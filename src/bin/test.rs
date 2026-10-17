use std::io::{self, Write};
use std::env;
use rand::{ rng, RngExt };
use loonn::core::{ 
    BirdDealer, Evaluator, Goal, GoalDealer, Player, ScoringEvent, Triggers, HABITATS
};
use loonn::simulation::{
    SimulatedBirdDeck, 
    SimulatedBonusCardDeck, 
    SimulatedFeeder, 
    SimulatedGoalDeck
};
use loonn::evaluators::{
    Randy,
    LooNN,
};

pub enum Evaluators {
    Randy(Randy),
    LooNN(LooNN),
}

impl Evaluator for Evaluators {
    fn evaluate(&mut self, player: &Player) -> f32 {
        match self {
            Evaluators::Randy(randy) => randy.evaluate(player),
            Evaluators::LooNN(loonn) => loonn.evaluate(player),
        }
    }

    fn evaluate_all(&mut self, players: &[Player]) -> Vec<f32> {
        match self {
            Evaluators::Randy(randy) => randy.evaluate_all(players),
            Evaluators::LooNN(loonn) => loonn.evaluate_all(players),
        }
    }

    fn choose<T>(
        &mut self,
        options: impl Iterator<Item = (T, Player)>,
        _played: bool,
    ) -> Option<(T, Player)> {       
        match self {
            Evaluators::Randy(randy) => randy.choose(options, false),
            Evaluators::LooNN(loonn) => loonn.choose(options, false),
        }
    }
}

pub enum EvaluatorType {
    Randy,
    LooNN,
}

struct Bot {
    player: Player,
    evaluator: Evaluators,
    triggers: Triggers,
}

impl Bot {
    fn new(
        eval_type: EvaluatorType,
        goals: [&'static Goal; 4],
        bird_dealer: &mut SimulatedBirdDeck,
        bonus_dealer: &mut SimulatedBonusCardDeck,
    ) -> Self {
        let mut evaluator = match eval_type {
            EvaluatorType::Randy => Evaluators::Randy(Randy::new()),
            EvaluatorType::LooNN => Evaluators::LooNN(
                LooNN::new().expect("Failed to construct LooNN")
            ),
        };

        let player = Player::new(
            &mut evaluator,
            goals,
            bird_dealer,
            bonus_dealer,
            false,
        );

        Self {
            player,
            evaluator,
            triggers: Triggers::new(),
        }
    }

    fn play_turn(
        &mut self, 
        feeder: &mut SimulatedFeeder,
        bird_dealer: &mut SimulatedBirdDeck,
        bonus_dealer: &mut SimulatedBonusCardDeck,
        rng: &mut impl RngExt
    ) -> Result<Triggers, ScoringEvent> {
        let result = self.player.play_turn(
            &mut self.evaluator,
            feeder,
            bird_dealer, 
            bonus_dealer, 
            &self.triggers,
            rng,
            false,
        );
        self.triggers = Triggers::new();
        result
    }
}

fn loading_bar(current: usize, total: usize) {
    const WIDTH: usize = 100;
    let progress = (current as f32 / total as f32).min(1.0);
    let filled = (progress * WIDTH as f32).round() as usize;
    let empty = WIDTH - filled;
    let bar = format!("{}>{}", "=".repeat(filled), " ".repeat(empty));
    let percent = (progress * 100.0) as u32;
    print!("\r[{}] {:>3}% ({}/{})", bar, percent, current, total);
    let _ = io::stdout().flush();
}

/// Players per test game, matching training. Seats alternate between Randy and LooNN.
const PLAYER_COUNT: usize = 4;

/// Plays one game between Randy and LooNN, returning each one's average score.
/// `max_turn`: if `Some(n)`, the game is truncated after each bot has taken
/// roughly `n` turns (matching the training-side truncation). If `None`,
/// the game plays out naturally to `EndOfGame`.
fn run_game(max_turn: Option<usize>) -> (f32, f32) {
    let mut bird_dealer = SimulatedBirdDeck::new(false);
    let mut bonus_dealer = SimulatedBonusCardDeck::new(false);
    let mut goal_dealer = SimulatedGoalDeck::new(false);
    let mut feeder = SimulatedFeeder::new(false);
    let goals = goal_dealer.set_goals();
    let mut rng = rng(); 

    // Randomize who sits first so neither evaluator always gets the first turn
    let randy_seat = rng.random_range(0..2);
    let is_randy = |i: usize| i % 2 == randy_seat;

    let mut bots: [Bot; PLAYER_COUNT] = std::array::from_fn(|i| {
        Bot::new(
            if is_randy(i) { EvaluatorType::Randy } else { EvaluatorType::LooNN },
            goals.clone(),
            &mut bird_dealer,
            &mut bonus_dealer,
        )
    });   
    
    let mut i: usize = 0;
    let mut n: usize = 0;
    let mut game_end: usize = 0;
    let turn_cap = max_turn.map(|t| t * PLAYER_COUNT);

    'outer: while game_end < PLAYER_COUNT {
        let mut round_end: usize = 0; 
        while round_end < PLAYER_COUNT {
            if let Some(cap) = turn_cap {
                if n >= cap {
                    break 'outer;
                }
            }

            bird_dealer.refill_face_up_birds();
            for h in HABITATS { 
                bots[i].triggers.fewest_birds.set_habitat(h, 
                    bots.iter()
                        .find(|&b| b.player.habitat(h).len() < bots[i].player.habitat(h).len())
                        .is_none()
                );
            }

            match bots[i].play_turn(
                &mut feeder,
                &mut bird_dealer, 
                &mut bonus_dealer,
                &mut rng,
            ) {
                Ok(triggers) => {
                    for (j, bot) in bots.iter_mut().enumerate() {
                        if i == j { continue; }
                        bot.triggers += triggers.clone();
                    }
                },
                Err(ScoringEvent::EndOfRound) => {
                    round_end += 1;
                    bots[i].triggers = Triggers::new();
                }
                Err(ScoringEvent::EndOfGame) => {
                    round_end += 1; 
                    game_end += 1;
                    bots[i].triggers = Triggers::new();
                },
            }
            i += 1;
            i %= PLAYER_COUNT;
            n += 1;
        }
        i += 1;
        i %= PLAYER_COUNT;
    }
    let average = |randy: bool| -> f32 {
        let scores: Vec<usize> = bots.iter()
            .enumerate()
            .filter(|&(i, _)| is_randy(i) == randy)
            .map(|(_, b)| b.player.accumulated_score())
            .collect();
        scores.iter().sum::<usize>() as f32 / scores.len() as f32
    };
    (average(true), average(false))
}

fn evaluate(games: usize, max_turn: Option<usize>) {
    let mut randy: f32 = 0.0;
    let mut loonn: f32 = 0.0;
    for i in 0..games {
        loading_bar(i, games);
        let (r, l) = run_game(max_turn);
        randy += r;
        loonn += l;
    }
    print!("\r{}\r", " ".repeat(130));
    let _ = io::stdout().flush();

    match max_turn {
        Some(t) => println!("--- {} turn(s)/player, {} {}-player games ---", t, games, PLAYER_COUNT),
        None => println!("--- Full games, {} {}-player games ---", games, PLAYER_COUNT),
    }
    println!("Randy averaged {} points", randy / games as f32);
    println!("LooNN averaged {} points", loonn / games as f32);
    let m = (loonn / games as f32) - (randy / games as f32);
    if m > 0.0 { println!("Average Difference: +{m}"); } 
    else { println!("Average Difference: {m}"); }
    println!();
}

/// Usage: test [full | turn cap[,turn cap...]] [games]
fn main() { 
    let args: Vec<String> = env::args().collect();

    let games: usize = args.get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    match args.get(1) {
        None => evaluate(games, None),
        Some(spec) if spec == "full" => evaluate(games, None),
        Some(spec) => {
            for part in spec.split(',') {
                let max_turn: usize = part.trim().parse()
                    .expect("turn cap must be `full`, a positive integer, or a comma-separated list of them");
                evaluate(games, Some(max_turn));
            }
        }
    }
}
