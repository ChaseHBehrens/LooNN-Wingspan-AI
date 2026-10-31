use rand::{ rng, RngExt, rngs::ThreadRng };
use std::fs::OpenOptions;
use std::io::Write;
use std::mem;
use std::sync::Mutex;
use std::sync::atomic::{ AtomicUsize, Ordering };
use std::sync::mpsc::{ channel, Receiver, Sender };
use std::thread;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use crate::core::{ Evaluator, Player };
use super::features::vectorize_player;

/// Player states packed for Python as little-endian f32 bytes
#[derive(Default)]
struct Packed {
    boards: Vec<u8>,
    cards: Vec<u8>,
    hand_lens: Vec<usize>,
}

impl Packed {
    fn new(players: &[Player]) -> Self {
        let mut packed = Self::default();
        for player in players {
            let (board, hand) = vectorize_player(player);
            packed.boards.extend(board.iter().flat_map(|f| f.to_le_bytes()));
            packed.cards.extend(hand.iter().flatten().flat_map(|f| f.to_le_bytes()));
            packed.hand_lens.push(hand.len());
        }
        packed
    }

    fn append(&mut self, other: Packed) {
        self.boards.extend(other.boards);
        self.cards.extend(other.cards);
        self.hand_lens.extend(other.hand_lens);
    }

    fn len(&self) -> usize { self.hand_lens.len() }
}

/// Work for the inference thread, which owns the Python model
enum Request {
    Evaluate { states: Packed, reply: Sender<Vec<f32>> },
    Train { states: Packed, scores: Vec<usize>, reply: Sender<()> },
}

/// Shared by every LooNN in the process so that games on different threads
/// are evaluated together in one batch
static SERVER: Mutex<Option<Sender<Request>>> = Mutex::new(None);
static SCORE_LOG: Mutex<()> = Mutex::new(());

/// Player-games already trained when the current schedules began. The first run
/// used noise alone and settled into hoarding food; the schedules below restart here.
const SCHEDULE_START_PLAYER_GAMES: usize = 9_101;
/// Noise on every choice, in points, at the start of the schedule
const EXPLORATION_START: f32 = 0.5;
/// Noise on every choice, in points, once the schedule finishes
const EXPLORATION_END: f32 = 0.2;
/// Chance of a random kind of action on a turn, at the start of the schedule
const RANDOM_ACTION_START: f32 = 0.15;
/// Chance of a random kind of action on a turn, once the schedule finishes
const RANDOM_ACTION_END: f32 = 0.05;
/// Player-games over which noise and random actions fall linearly from start to end
/// (about 10 hours of 4 player games at 8 games in parallel)
const EXPLORATION_PLAYER_GAMES: usize = 10_000;
/// Points added per bird on the mat when choosing, in training and in play. The
/// network undervalues birds, and fading bonuses taught it only about 1 point of
/// that. At 130k player-games, 100 games against Randy at each bonus scored 62.0
/// (none), 66.1 (1), 66.8 (2), 65.6 (3) and 68.0 (4), so the bonus is permanent.
/// Only choices are biased, never the scores it learns from.
const BIRD_BONUS: f32 = 2.0;
const SCORE_LOG_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/neural_network/game_scores.csv");

/// Training player-games finished so far, including earlier runs from the score log
static PLAYER_GAMES: AtomicUsize = AtomicUsize::new(0);

/// Scale of the exploration noise. Falls on a fixed schedule over the number of
/// player-games trained, so the training games follow the network's own choices
/// more and more closely.
pub fn exploration() -> f32 {
    let progress = schedule_progress(EXPLORATION_PLAYER_GAMES);
    EXPLORATION_START + (EXPLORATION_END - EXPLORATION_START) * progress
}

/// Chance of a random kind of action on a turn. Falls on the same schedule as `exploration()`.
pub fn random_action_rate() -> f32 {
    let progress = schedule_progress(EXPLORATION_PLAYER_GAMES);
    RANDOM_ACTION_START + (RANDOM_ACTION_END - RANDOM_ACTION_START) * progress
}

/// Points added per bird on the mat when choosing
pub fn bird_bonus() -> f32 { BIRD_BONUS }

/// Fraction of `length` player-games trained since the schedules began, capped at 1
fn schedule_progress(length: usize) -> f32 {
    let trained = PLAYER_GAMES.load(Ordering::Relaxed).saturating_sub(SCHEDULE_START_PLAYER_GAMES);
    (trained as f32 / length as f32).min(1.0)
}

/// Picks up the player-game count from the score log so a restarted run keeps its place in the schedules
fn load_player_games() {
    let Ok(log) = std::fs::read_to_string(SCORE_LOG_PATH) else { return };
    PLAYER_GAMES.store(log.lines().count().saturating_sub(1), Ordering::Relaxed);
}

fn connect() -> PyResult<Sender<Request>> {
    let mut server = SERVER.lock().unwrap();
    if let Some(sender) = server.as_ref() { return Ok(sender.clone()); }
    let py_module = Python::attach(|py| -> PyResult<Py<PyModule>> {
        let sys = PyModule::import(py, "sys")?;
        let nn_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/neural_network");
        sys.getattr("path")?.call_method1("append", (nn_dir,))?;
        let module = PyModule::import(py, "loonn")?;
        module.getattr("init")?.call0()?;
        Ok(module.unbind())
    })?;
    load_player_games();
    let (sender, requests) = channel();
    // Python and PyTorch run on this thread; the default 2 MB stack overflowed once
    thread::Builder::new()
        .name("loonn-inference".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || serve(py_module, requests))
        .expect("failed to spawn LooNN inference thread");
    *server = Some(sender.clone());
    Ok(sender)
}

/// Answers requests until every sender is gone, merging all queued
/// evaluations into a single Python call
fn serve(py_module: Py<PyModule>, requests: Receiver<Request>) {
    while let Ok(first) = requests.recv() {
        let mut evaluations = Vec::<(Packed, Sender<Vec<f32>>)>::new();
        for request in std::iter::once(first).chain(requests.try_iter()) {
            match request {
                Request::Evaluate { states, reply } => {
                    evaluations.push((states, reply));
                },
                Request::Train { states, scores, reply } => {
                    // Answer earlier requests with the weights they were asked against
                    evaluate_batched(&py_module, mem::take(&mut evaluations));
                    Python::attach(|py| {
                        py_module
                            .bind(py)
                            .getattr("train")
                            .expect("loonn.py missing `train` function")
                            .call1((
                                states.boards.as_slice(), 
                                states.cards.as_slice(), 
                                states.hand_lens, 
                                scores,
                            ))
                            .expect("python train() call failed");
                    });
                    let _ = reply.send(());
                },
            }
        }
        evaluate_batched(&py_module, evaluations);
    }
}

fn evaluate_batched(
    py_module: &Py<PyModule>, 
    evaluations: Vec<(Packed, Sender<Vec<f32>>)>,
) {
    if evaluations.is_empty() { return; }
    let mut all = Packed::default();
    let mut replies = Vec::<(usize, Sender<Vec<f32>>)>::new();
    for (states, reply) in evaluations {
        replies.push((states.len(), reply));
        all.append(states);
    }
    let mut predictions = Python::attach(|py| {
        py_module
            .bind(py)
            .getattr("evaluate_packed")
            .expect("loonn.py missing `evaluate_packed` function")
            .call1((all.boards.as_slice(), all.cards.as_slice(), all.hand_lens))
            .expect("python evaluate_packed() call failed")
            .extract::<Vec<f32>>()
            .expect("evaluate_packed() must return a list of floats")
    }).into_iter();
    for (len, reply) in replies {
        let _ = reply.send(predictions.by_ref().take(len).collect());
    }
}

/// Evaluates a player using a neural network
pub struct LooNN {
    history: Vec<Player>,
    evaluation_sum: f32,
    server: Sender<Request>,
    rng: ThreadRng,
    explore: bool,
    /// Points per bird on the mat added when choosing outside training, for testing
    /// whether the network undervalues birds
    fixed_bird_bonus: f32,
}

impl LooNN {
    /// Plays greedily, always choosing the highest evaluation plus the current bird bonus
    pub fn new() -> PyResult<Self> {
        // Connecting reads the score log, which the bird bonus depends on
        let server = connect()?;
        Ok(Self {
            history: Vec::new(),
            evaluation_sum: 0.0,
            server,
            rng: rng(),
            explore: false,
            fixed_bird_bonus: bird_bonus(),
        })
    }

    /// Plays greedily, adding `bonus` points per bird on the mat when choosing instead of the current bird bonus
    pub fn with_bird_bonus(bonus: f32) -> PyResult<Self> {
        Ok(Self { fixed_bird_bonus: bonus, ..Self::new()? })
    }

    /// Adds exploration noise to evaluations, for training
    pub fn new_exploring() -> PyResult<Self> {
        Ok(Self { explore: true, ..Self::new()? })
    }

    pub fn train(&self, score: usize) {
        let states = Packed::new(&self.history);
        let scores: Vec<usize> = self.history
            .iter()
            .map(|p| score.saturating_sub(p.accumulated_score()))
            .collect();
        self.log_final_score(score); 
        PLAYER_GAMES.fetch_add(1, Ordering::Relaxed);
        let (reply, done) = channel();
        self.server
            .send(Request::Train { states, scores, reply })
            .expect("LooNN inference thread stopped");
        done.recv().expect("LooNN inference thread stopped");
    }
    
    fn log_final_score(&self, score: usize) {
        let _lock = SCORE_LOG.lock().unwrap();
        let path = SCORE_LOG_PATH;
        let file_exists = std::path::Path::new(path).exists();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("failed to open game_scores.csv");
        if !file_exists {
            writeln!(file, "game,score").expect("failed to write csv header");
        }
        let game_num = std::fs::read_to_string(path)
            .map(|s| s.lines().count().saturating_sub(1))
            .unwrap_or(0);
        writeln!(file, "{},{}", game_num, score).expect("failed to write score row");
    }

    pub fn average_evaluation(&self) -> f32 {
        if self.history.len() == 0 { 0.0 }
        else { self.evaluation_sum / self.history.len() as f32 }
    }

    /// Predicted remaining score for each player, without noise
    fn predict(&self, players: &[Player]) -> Vec<f32> {
        if players.is_empty() { return Vec::new(); }
        let (reply, predictions) = channel();
        self.server
            .send(Request::Evaluate { states: Packed::new(players), reply })
            .expect("LooNN inference thread stopped");
        predictions.recv().expect("LooNN inference thread stopped")
    }
}

impl Evaluator for LooNN {
    fn evaluate(&mut self, player: &Player) -> f32 {
        let prediction = self.predict(std::slice::from_ref(player))[0];
        self.finalize_score(prediction, player)
    }

    fn evaluate_all(&mut self, players: &[Player]) -> Vec<f32> {
        self.predict(players)
            .into_iter()
            .zip(players)
            .map(|(prediction, p)| self.finalize_score(prediction, p))
            .collect()
    }

    fn choose<T>(
        &mut self,
        options: impl Iterator<Item = (T, Player)>,
        played: bool,
    ) -> Option<(T, crate::core::Player)> {
        let (choices, players): (Vec<T>, Vec<Player>) = options.unzip();
        let scores = self.evaluate_all(&players);
        let (score, t, p) = scores.into_iter()
            .zip(choices)
            .zip(players)
            .map(|((score, t), p)| (score, t, p))
            .max_by(|a, b| a.0.total_cmp(&b.0))?;
        if played {
            self.evaluation_sum += score - self.bias(&p);
            self.history.push(p.clone());
        }
        Some((t, p))
    }

    fn random_action_rate(&self) -> f32 {
        if self.explore { random_action_rate() } else { 0.0 }
    }
}

impl LooNN {
    /// Turns a predicted remaining score into a noisy predicted final score
    fn finalize_score(&mut self, score: f32, player: &Player) -> f32 {
        // Gumbel noise makes the max a softmax sample with temperature `exploration()`
        let noise = if self.explore {
            let u: f32 = self.rng.random_range(1e-6..1.0);
            exploration() * (-(-u.ln()).ln())
        } else { 0.0 };
        let noisy_score = score + noise;
        noisy_score.max(0.0) + player.accumulated_score() as f32 + self.bias(player)
    }

    /// Bonus for birds on the mat while training, which steers choices but not predictions
    fn bias(&self, player: &Player) -> f32 {
        let per_bird = if self.explore { bird_bonus() } else { self.fixed_bird_bonus };
        per_bird * player.bird_slots().count() as f32
    }
}
