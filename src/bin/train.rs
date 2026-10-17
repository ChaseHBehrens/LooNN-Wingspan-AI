use std::env;
use std::fmt::Write;
use std::sync::atomic::{ AtomicUsize, Ordering };
use std::thread;
use rand::{ rng, RngExt };
use loonn::core::{ 
    BirdDealer, 
    Goal, GoalDealer, 
    Player, 
    ScoringEvent, 
    Triggers, 
    HABITATS,
};
use loonn::simulation::{
    SimulatedBirdDeck, 
    SimulatedBonusCardDeck, 
    SimulatedFeeder, 
    SimulatedGoalDeck
};
use loonn::evaluators::{ LooNN, exploration };


const PLAYER_COUNT: usize = 4;
const LOGGED: bool = false;
/// Stack size of each game thread
const GAME_STACK_SIZE: usize = 64 * 1024 * 1024;

/// Games played at once when no count is given on the command line
fn default_parallel_games() -> usize {
    thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

struct Bot {
    player: Player,
    evaluator: LooNN,
    triggers: Triggers,
    logged: bool,
}

impl Bot {
    fn new(
        goals: [&'static Goal; 4],
        bird_dealer: &mut SimulatedBirdDeck,
        bonus_dealer: &mut SimulatedBonusCardDeck,
        logged: bool,
    ) -> Self {
        let mut evaluator = LooNN::new_exploring()
            .expect("Failed to construct LooNN");
        let player = Player::new(
            &mut evaluator,
            goals,
            bird_dealer,
            bonus_dealer,
            logged,
        );
        Self {
            player,
            evaluator,
            triggers: Triggers::new(),
            logged,
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
            self.logged,
        );
        self.triggers = Triggers::new();
        result
    }
}

fn run_game(game: usize) {
    let mut bird_dealer = SimulatedBirdDeck::new(LOGGED);
    let mut bonus_dealer = SimulatedBonusCardDeck::new(LOGGED);
    let mut goal_dealer = SimulatedGoalDeck::new(LOGGED);
    let mut feeder = SimulatedFeeder::new(LOGGED);
    let goals = goal_dealer.set_goals();
    let mut rng = rng(); 
    let mut bots: [Bot; PLAYER_COUNT] = std::array::from_fn(|i| {
        if LOGGED { println!("\x1b[35m=\x1b[0m Player {} Setup", i + 1); }
        Bot::new(goals.clone(), &mut bird_dealer, &mut bonus_dealer, LOGGED)
    });   
    
    let mut i: usize = 0;
    let mut game_end: usize = 0;
    while game_end < PLAYER_COUNT {
        if LOGGED { println!("\x1b[35m=\x1b[0m New Round"); }
        let mut round_end: usize = 0; 
        while round_end < PLAYER_COUNT {
            if LOGGED { println!("\x1b[35m=\x1b[0m Player {}", i + 1); }
            bird_dealer.refill_face_up_birds();
            for h in HABITATS { bots[i].triggers.fewest_birds.set_habitat(h, 
                bots.iter()
                    .find(|&b| b.player.habitat(h).len() < bots[i].player.habitat(h).len())
                    .is_none()
            );}
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
        }
        i += 1;
        i %= PLAYER_COUNT;
    }

    // Printed as one block so parallel games don't interleave
    let mut summary = format!("Game {game} (exploration {:.2})\n", exploration());
    for (i, bot) in bots.iter().enumerate() {
        let score = bot.player.accumulated_score();
        let _ = writeln!(summary, "Player {} scored {} points.", i + 1, score);
        let evaluation = bot.evaluator.average_evaluation();
        let _ = writeln!(summary, "LooNN predicted player {} would score {:.1} points.", i + 1, evaluation);
    }
    print!("{summary}");
    for bot in bots.iter() {
        bot.evaluator.train(bot.player.accumulated_score());
    }
}


/// Usage: train [parallel games]
fn main() { 
    let parallel_games: usize = env::args().nth(1)
        .map(|s| s.parse().expect("parallel games must be a positive integer"))
        .unwrap_or_else(default_parallel_games)
        .max(1);
    println!("Running {parallel_games} game(s) in parallel");
    let next_game = AtomicUsize::new(1);
    thread::scope(|scope| {
        for _ in 0..parallel_games {
            // The default 2 MB thread stack overflowed once in an overnight run
            thread::Builder::new()
                .stack_size(GAME_STACK_SIZE)
                .spawn_scoped(scope, || loop {
                    run_game(next_game.fetch_add(1, Ordering::Relaxed));
                })
                .expect("failed to spawn game thread");
        }
    });
}
