use rand::{ rng, RngExt };
use loonn::core::{ 
    BirdDealer, 
    Goal, GoalDealer, 
    Player, 
    ScoringEvent, 
    Triggers, 
    HABITATS,
    Evaluator,
};
use loonn::simulation::{
    SimulatedBirdDeck, 
    SimulatedBonusCardDeck, 
    SimulatedFeeder, 
    SimulatedGoalDeck
};
use loonn::evaluators::LooNN;


const PLAYER_COUNT: usize = 1;
const LOGGED: bool = true;


struct Bot {
    player: Player,
    evaluator: LooNN,
    triggers: Triggers,
}

impl Bot {
    fn new(
        goals: [&'static Goal; 4],
        bird_dealer: &mut SimulatedBirdDeck,
        bonus_dealer: &mut SimulatedBonusCardDeck,
    ) -> Self {
        let mut evaluator = LooNN::new()
            .expect("Failed to construct LooNN");
        let player = Player::new(
            &mut evaluator,
            goals,
            bird_dealer,
            bonus_dealer,
            true,
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
            true,
        );
        self.triggers = Triggers::new();
        result
    }
}


fn run_game() {
    let mut bird_dealer = SimulatedBirdDeck::new(true);
    let mut bonus_dealer = SimulatedBonusCardDeck::new(true);
    let mut goal_dealer = SimulatedGoalDeck::new(true);
    let mut feeder = SimulatedFeeder::new(true);
    let goals = goal_dealer.set_goals();
    let mut rng = rng(); 
    let mut bots: [Bot; PLAYER_COUNT] = std::array::from_fn(|i| {
        if LOGGED { println!("\x1b[35m=\x1b[0m Player {} Setup", i + 1); }
        Bot::new(goals.clone(), &mut bird_dealer, &mut bonus_dealer)
    });   
    
    let mut i: usize = 0;
    let mut game_end: usize = 0;
    while game_end < PLAYER_COUNT {
        if LOGGED { println!("\x1b[35m=\x1b[0m New Round"); }
        let mut round_end: usize = 0; 
        while round_end < PLAYER_COUNT {
            println!("\x1b[35m=\x1b[0m Player {}", i + 1);
            
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
            println!("\x1b[36m~\x1b[0m {:.1}", bots[i].evaluator.evaluate(&bots[i].player));
            i += 1;
            i %= PLAYER_COUNT;
        }
        i += 1;
        i %= PLAYER_COUNT;
    }

    for (i, bot) in bots.iter().enumerate() {
        let score = bot.player.accumulated_score();
        println!("Player {} scored {} points.", i + 1, score);
        let evaluation = bot.evaluator.average_evaluation();
        println!("LooNN predicted player {} would score {:.1} points.", i + 1, evaluation);
    }
}


fn main() { run_game(); }

