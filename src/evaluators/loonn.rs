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
use crate::core::{ 
    Evaluator, 
    Cost,
    FoodCollection, 
    HabitatCollection, HABITATS,
    Player, 
    Power,
    Nest, 
    Niche,
    Bird, BirdSlot,
    Activation, Trigger,
    Position,
};

type Board = Vec<f32>;
type Hand = Vec<Vec<f32>>;

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

/// Exploration noise, in points, at the start of training
const EXPLORATION_START: f32 = 1.0;
/// Exploration noise, in points, once the schedule finishes
const EXPLORATION_END: f32 = 0.2;
/// Player-games over which exploration falls linearly from start to end
/// (about 3.5 hours of 4 player games at 8 games in parallel)
const EXPLORATION_PLAYER_GAMES: usize = 8_000;
const SCORE_LOG_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/neural_network/game_scores.csv");

/// Training player-games finished so far, including earlier runs from the score log
static PLAYER_GAMES: AtomicUsize = AtomicUsize::new(0);

/// Scale of the exploration noise. Falls on a fixed schedule over the number of
/// player-games trained, so the training games follow the network's own choices
/// more and more closely.
pub fn exploration() -> f32 {
    let progress = PLAYER_GAMES.load(Ordering::Relaxed) as f32 / EXPLORATION_PLAYER_GAMES as f32;
    EXPLORATION_START + (EXPLORATION_END - EXPLORATION_START) * progress.min(1.0)
}

/// Picks up the player-game count from the score log so a restarted run keeps its place in the schedule
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
    thread::spawn(move || serve(py_module, requests));
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
}

impl LooNN {
    /// Plays greedily, always choosing the highest evaluation
    pub fn new() -> PyResult<Self> {
        Ok(Self {
            history: Vec::new(),
            evaluation_sum: 0.0,
            server: connect()?,
            rng: rng(),
            explore: false,
        })
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
            self.history.push(p.clone());
            self.evaluation_sum += score;
        }
        Some((t, p))
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
        // TEST: try adding a bias towards having more birds
        //let bias: f32 = HABITATS.into_iter()
        //    .map(|h| {
        //        match player.habitat(h).len() {
        //            1 => 5.0,
        //            2 => 10.0,
        //            3 => 15.0,
        //            4 => 20.0,
        //            5 => 25.0,
        //            _ => 0.0
        //        }
        //    })
        //    .sum::<f32>();
        //let biased_score = noisy_score + bias;
        noisy_score.max(0.0) + player.accumulated_score() as f32
    }
}


fn vectorize_power(power: &Power) -> Vec<f32> {
    let mut v = Vec::<f32>::new();
    v.push(power.play_bird_in_this_habitat.into());
    v.extend(vectorize_habitat_collection(&power.play_bird));
    v.push(power.migration.into());
    if let Some(bird) = power.hunt_bird {
        v.push(1.0);
        v.push(bird as f32 / 300.0);
    } else {
        v.extend(vec![0.0, 0.0]);
    }
    v.extend(vectorize_food_constraint(&power.food_constraint));
    v.push(power.hunt_dice.into());
    v.push(power.feeder.into());
    v.push(power.gain_food as f32 / 2.0);
    v.push(power.discard_food.into());
    v.push(power.self_constraint.into());
    v.extend(vectorize_nest(&power.nest_constraint));
    v.push(power.lay_eggs_on_all.into());
    v.push(power.lay_eggs as f32 / 2.0);
    v.push(power.discard_egg.into());
    v.extend(vectorize_habitat_collection(&power.fewest_birds.into()));
    v.push(power.face_up_constraint.into());
    v.push(power.deck_constraint.into());
    v.push(power.draw_bird as f32 / 2.0);
    v.push(power.tuck_or_cache as f32 / 2.0);
    v.push(power.discard_bird.into());
    if let Some(niche) = power.repeat_power {
        v.push(1.0);
        v.extend(vectorize_niche(&niche))
    } else { v.extend(vec![0.0, 0.0, 0.0]); }
    v.push(power.draw_bonus.into());
    v
}

fn vectorize_habitat_collection(habitat_collection: &HabitatCollection) -> Vec<f32> {
    vec![
        habitat_collection.forest.into(),
        habitat_collection.grassland.into(),
        habitat_collection.wetland.into(),
    ]
}

fn vectorize_food_constraint(food: &FoodCollection) -> Vec<f32> {
    vec![
        food.invertebrates as f32,
        food.seeds as f32,
        food.fish as f32,
        food.fruits as f32,
        food.rodents as f32,
    ]
}

fn vectorize_food_collection(food: &FoodCollection) -> Vec<f32> {
    vec![
        food.invertebrates as f32 / 2.0,
        food.seeds as f32 / 2.0,
        food.fish as f32 / 2.0,
        food.fruits as f32 / 2.0,
        food.rodents as f32 / 2.0,
        food.wilds as f32 / 2.0,
    ]
}

/// Food that must be paid, followed by flags for foods that can be paid as 1 of
fn vectorize_cost(cost: &Cost) -> Vec<f32> {
    match cost {
        Cost::All(food) => [vectorize_food_collection(food), vec![0.0; 6]].concat(),
        Cost::OneOf(foods) => [
            vec![0.0; 6],
            vectorize_food_collection(foods).into_iter().map(|f| (f > 0.0).into()).collect(),
        ].concat(),
    }
}

fn vectorize_food_supply(food: &FoodCollection) -> Vec<f32> {
    vec![
        (food.invertebrates as f32 / 15.0).clamp(0.0, 1.0),
        (food.seeds as f32 / 15.0).clamp(0.0, 1.0),
        (food.fish as f32 / 15.0).clamp(0.0, 1.0),
        (food.fruits as f32 / 15.0).clamp(0.0, 1.0),
        (food.rodents as f32 / 15.0).clamp(0.0, 1.0),
    ]
    
}

fn vectorize_nest(nest: &Nest) -> Vec<f32> {
    match nest {
        Nest::Platform => vec![1.0, 0.0, 0.0, 0.0],
        Nest::Bowl     => vec![0.0, 1.0, 0.0, 0.0],
        Nest::Cavity   => vec![0.0, 0.0, 1.0, 0.0],
        Nest::Ground   => vec![0.0, 0.0, 0.0, 1.0],
        Nest::Wild     => vec![1.0, 1.0, 1.0, 1.0],
        Nest::None     => vec![0.0, 0.0, 0.0, 0.0],
    }
}

fn vectorize_niche(niche: &Niche) -> Vec<f32> {
    match niche {
        Niche::None     => vec![0.0, 0.0],
        Niche::Predator => vec![1.0, 0.0],
        Niche::Flocking => vec![0.0, 1.0],
        Niche::Any      => vec![1.0, 1.0],
    }
}

fn vectorize_trigger(trigger: &Trigger) -> Vec<f32> {
    let mut v = Vec::<f32>::new();
    match trigger {
        Trigger::LayEgg => {
            v.push(1.0);
            v.extend(vec![0.0; 3]);
            v.extend(vec![0.0; 5]);
            v.push(0.0);
        },
        Trigger::PlayBird(habitat) => {
            v.push(0.0);
            v.extend(vectorize_habitat_collection(&habitat.clone().into()));
            v.extend(vec![0.0; 5]);
            v.push(0.0);
        },
        Trigger::GainFood(food) => {
            v.push(0.0);
            v.extend(vec![0.0; 3]);
            v.extend(vectorize_food_constraint(&food.clone().into()));
            v.push(0.0);
        },
        Trigger::HuntSuccess => {
            v.push(0.0);
            v.extend(vec![0.0; 3]);
            v.extend(vec![0.0; 5]);
            v.push(1.0);
        },
    }
    v
}

fn vectorize_activation(activation: &Activation) -> Vec<f32> {
    let mut v = Vec::<f32>::new();
    match activation {
        Activation::None => {
            v.extend(vec![0.0, 0.0, 0.0]);
            v.extend(vec![0.0; 10]);
            v.extend(vec![0.0; 36]);
        },
        Activation::WhenActivated(power) => {
            v.extend(vec![1.0, 0.0, 0.0]);
            v.extend(vec![0.0; 10]);
            v.extend(vectorize_power(power));
        },
        Activation::OnceBetweenTurns(trigger, power) => {
            v.extend(vec![0.0, 1.0, 0.0]);
            v.extend(vectorize_trigger(trigger));
            v.extend(vectorize_power(power));
        },
        Activation::WhenPlayed(power) => {
            v.extend(vec![0.0, 0.0, 1.0]);
            v.extend(vec![0.0; 10]);
            v.extend(vectorize_power(power));
        },
    }
    v
}

/// Length of a vectorized bird
const BIRD_DIM: usize = 108;

fn vectorize_bird(bird: &Bird) -> Vec<f32> {
    let mut v = Vec::<f32>::new();
    v.extend(vectorize_cost(&bird.cost));
    v.push(bird.points as f32 / 9.0);
    v.extend(vectorize_niche(&bird.niche));
    v.extend(vectorize_habitat_collection(&bird.habitat));
    v.extend(vectorize_nest(&bird.nest));
    v.push(bird.egg_limit as f32 / 6.0);
    v.extend(vectorize_activation(&bird.power));
    if let Some(power) = bird.shared_power {
        v.extend(vectorize_power(&power))
    } else {
        v.extend(vec![0.0; 36])
    }
    debug_assert_eq!(v.len(), BIRD_DIM);
    v
}

/// One vector per card; the network pools these with attention
fn vectorize_hand(hand: &Vec<&'static Bird>) -> Hand {
    hand.iter().map(|bird| vectorize_bird(bird)).collect()
}

/// Length of a vectorized bird slot, including the occupied flag
const BIRD_SLOT_DIM: usize = 1 + BIRD_DIM + 2;

fn vectorize_bird_slot(slot: &BirdSlot) -> Vec<f32> {
    let mut v = Vec::<f32>::new();
    v.push(1.0);
    v.extend(vectorize_bird(slot.bird));
    v.push(slot.eggs() as f32 / 6.0);
    v.push((slot.bird.egg_limit - slot.eggs()) as f32 / 6.0);
    v
}

fn vectorize_position(player: &Player) -> Vec<f32> {
    let mut v = Vec::<f32>::new();
    match player.activating() {
        Position::Triggers => v.extend(vec![0.0; 8]),
        Position::Played(h) => {
            v.extend(vectorize_habitat_collection(&h.clone().into()));
            match player.habitat(*h).len() {
                1 => v.extend(vec![1.0, 0.0, 0.0, 0.0, 0.0]),
                2 => v.extend(vec![0.0, 1.0, 0.0, 0.0, 0.0]),
                3 => v.extend(vec![0.0, 0.0, 1.0, 0.0, 0.0]),
                4 => v.extend(vec![0.0, 0.0, 0.0, 1.0, 0.0]),
                5 => v.extend(vec![0.0, 0.0, 0.0, 0.0, 1.0]),
                _ => unreachable!("invalid index")
            }
        },
        Position::Chain((h, i)) => {
            v.extend(vectorize_habitat_collection(&h.clone().into()));
            match i {
                0 => v.extend(vec![1.0, 0.0, 0.0, 0.0, 0.0]),
                1 => v.extend(vec![0.0, 1.0, 0.0, 0.0, 0.0]),
                2 => v.extend(vec![0.0, 0.0, 1.0, 0.0, 0.0]),
                3 => v.extend(vec![0.0, 0.0, 0.0, 1.0, 0.0]),
                4 => v.extend(vec![0.0, 0.0, 0.0, 0.0, 1.0]),
                _ => unreachable!("invalid index")
            }
        },
    }
    v
}

/// Returns the board vector and the per-card hand vectors
fn vectorize_player(player: &Player) -> (Board, Hand) { 
    let mut v = Vec::<f32>::new();
    for h in HABITATS { for i in 0..5 {
        if let Some(s) = player.habitat(h).get(i) {
            v.extend(vectorize_bird_slot(s));
        } else {
            v.extend(vec![0.0; BIRD_SLOT_DIM])
        }
    }}
    v.extend(vectorize_food_supply(&player.food()));
    v.extend(vectorize_habitat_collection(&player.fewest_birds()));
    v.push((player.hand().len() as f32 / 30.0).clamp(0.0, 1.0));
    v.push(player.bonus_cards()
        .iter()
        .map(|&b| b.score(player))
        .sum::<usize>() 
        as f32 / 80.0
    );
    let total_remaining_turns = player.total_remaining_turns();    
    v.extend(player.goals()
        .iter()
        .zip([18, 11, 5, 0])
        .map(|(&g, t)| if total_remaining_turns > t { 
            g.score(player) as f32 / 5.0 
        } else { 0.0 })
    );
    v.extend(vectorize_position(player));
    for turn in 1..=26 { v.push(if turn == total_remaining_turns { 1.0 } else { 0.0 }); }
    (v, vectorize_hand(player.hand()))
}

