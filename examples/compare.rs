//! Diagnostic: plays LooNN against Randy and breaks down where each one's points
//! come from and which kind of turn each one takes.
//!
//! Usage: cargo run --release --example compare [games] [bird bonus]

use std::env;
use rand::{ rng, RngExt };
use loonn::core::{
    BirdDealer, Evaluator, Goal, GoalDealer, Player, ScoringEvent, Triggers, HABITATS,
};
use loonn::simulation::{
    SimulatedBirdDeck, SimulatedBonusCardDeck, SimulatedFeeder, SimulatedGoalDeck,
};
use loonn::evaluators::{ Randy, LooNN };

enum Evaluators { Randy(Randy), LooNN(LooNN) }

impl Evaluator for Evaluators {
    fn evaluate(&mut self, player: &Player) -> f32 {
        match self { Self::Randy(e) => e.evaluate(player), Self::LooNN(e) => e.evaluate(player) }
    }
    fn evaluate_all(&mut self, players: &[Player]) -> Vec<f32> {
        match self { Self::Randy(e) => e.evaluate_all(players), Self::LooNN(e) => e.evaluate_all(players) }
    }
    fn choose<T>(&mut self, options: impl Iterator<Item = (T, Player)>, _played: bool) -> Option<(T, Player)> {
        match self { Self::Randy(e) => e.choose(options, false), Self::LooNN(e) => e.choose(options, false) }
    }
}

#[derive(Default, Clone)]
struct Stats {
    players: f32,
    score: f32,
    bird_points: f32,
    other_turn_points: f32,
    goal_points: f32,
    egg_points: f32,
    bonus_points: f32,
    birds: f32,
    food_left: f32,
    hand_left: f32,
    turns_play: f32,
    turns_eggs: f32,
    turns_food: f32,
    turns_draw: f32,
    turns_other: f32,
    first_bird_turn: f32,
    initial_food: f32,
    initial_hand: f32,
}

fn total_eggs(p: &Player) -> usize { p.bird_slots().map(|s| s.eggs() as usize).sum() }
fn food_count(p: &Player) -> usize {
    let f = p.food();
    (f.invertebrates + f.seeds + f.fish + f.fruits + f.rodents + f.wilds) as usize
}
fn bird_count(p: &Player) -> usize { p.bird_slots().count() }

fn run_game(randy_stats: &mut Stats, loonn_stats: &mut Stats) {
    let mut bird_dealer = SimulatedBirdDeck::new(false);
    let mut bonus_dealer = SimulatedBonusCardDeck::new(false);
    let mut goal_dealer = SimulatedGoalDeck::new(false);
    let mut feeder = SimulatedFeeder::new(false);
    let goals: [&'static Goal; 4] = goal_dealer.set_goals();
    let mut rng = rng();
    let randy_seat = rng.random_range(0..2);
    let is_randy = |i: usize| i % 2 == randy_seat;

    let mut evaluators: Vec<Evaluators> = (0..4).map(|i| if is_randy(i) {
        Evaluators::Randy(Randy::new())
    } else {
        Evaluators::LooNN(match bonus_arg() {
            Some(bonus) => LooNN::with_bird_bonus(bonus),
            None => LooNN::new(),
        }.expect("Failed to construct LooNN"))
    }).collect();
    let mut players: Vec<Player> = evaluators.iter_mut()
        .map(|e| Player::new(e, goals, &mut bird_dealer, &mut bonus_dealer, false))
        .collect();
    let mut triggers: Vec<Triggers> = (0..4).map(|_| Triggers::new()).collect();
    let mut stats: Vec<Stats> = (0..4).map(|_| Stats::default()).collect();
    for (s, p) in stats.iter_mut().zip(&players) {
        s.initial_food = food_count(p) as f32;
        s.initial_hand = p.hand().len() as f32;
        s.first_bird_turn = 27.0;
    }

    let mut i = 0;
    let mut game_end = 0;
    while game_end < 4 {
        let mut round_end = 0;
        while round_end < 4 {
            bird_dealer.refill_face_up_birds();
            for h in HABITATS {
                let mine = players[i].habitat(h).len();
                let fewest = players.iter().all(|p| p.habitat(h).len() >= mine);
                triggers[i].fewest_birds.set_habitat(h, fewest);
            }
            let before = players[i].clone();
            let result = players[i].play_turn(
                &mut evaluators[i], &mut feeder, &mut bird_dealer, &mut bonus_dealer,
                &triggers[i], &mut rng, false,
            );
            // Triggers are used up by the turn, as in train.rs and test.rs
            triggers[i] = Triggers::new();
            let after = &players[i];
            let gained = (after.accumulated_score() - before.accumulated_score()) as f32;
            let s = &mut stats[i];
            match result {
                Ok(t) => {
                    let new_birds = bird_count(after) - bird_count(&before);
                    let bird_pts: usize = after.bird_slots().skip(0).map(|s| s.bird.points as usize).sum::<usize>()
                        - before.bird_slots().map(|s| s.bird.points as usize).sum::<usize>();
                    s.bird_points += bird_pts as f32;
                    s.other_turn_points += gained - bird_pts as f32;
                    if new_birds > 0 {
                        s.turns_play += 1.0;
                        if s.first_bird_turn > 26.0 { s.first_bird_turn = (26 - after.total_remaining_turns()) as f32; }
                    } else if total_eggs(after) > total_eggs(&before) { s.turns_eggs += 1.0; }
                    else if food_count(after) > food_count(&before) { s.turns_food += 1.0; }
                    else if after.hand().len() > before.hand().len() { s.turns_draw += 1.0; }
                    else { s.turns_other += 1.0; }
                    for (j, tr) in triggers.iter_mut().enumerate() {
                        if j != i { *tr += t.clone(); }
                    }
                }
                Err(ScoringEvent::EndOfRound) => {
                    s.goal_points += gained;
                    round_end += 1;
                    triggers[i] = Triggers::new();
                }
                Err(ScoringEvent::EndOfGame) => {
                    let eggs = total_eggs(after) as f32;
                    let bonus: usize = after.bonus_cards().iter().map(|b| b.score(after)).sum();
                    s.egg_points += eggs;
                    s.bonus_points += bonus as f32;
                    s.goal_points += gained - eggs - bonus as f32;
                    round_end += 1;
                    game_end += 1;
                    triggers[i] = Triggers::new();
                }
            }
            i = (i + 1) % 4;
        }
        i = (i + 1) % 4;
    }
    for (idx, (s, p)) in stats.iter_mut().zip(&players).enumerate() {
        s.players = 1.0;
        s.score = p.accumulated_score() as f32;
        s.birds = bird_count(p) as f32;
        s.food_left = food_count(p) as f32;
        s.hand_left = p.hand().len() as f32;
        let target = if is_randy(idx) { &mut *randy_stats } else { &mut *loonn_stats };
        add(target, s);
    }
}

fn add(a: &mut Stats, b: &Stats) {
    a.players += b.players; a.score += b.score; a.bird_points += b.bird_points;
    a.other_turn_points += b.other_turn_points; a.goal_points += b.goal_points;
    a.egg_points += b.egg_points; a.bonus_points += b.bonus_points; a.birds += b.birds;
    a.food_left += b.food_left; a.hand_left += b.hand_left; a.turns_play += b.turns_play;
    a.turns_eggs += b.turns_eggs; a.turns_food += b.turns_food; a.turns_draw += b.turns_draw;
    a.turns_other += b.turns_other; a.first_bird_turn += b.first_bird_turn;
    a.initial_food += b.initial_food; a.initial_hand += b.initial_hand;
}

/// Points per bird LooNN adds when choosing (second argument); without it LooNN
/// uses the current bird bonus, as in training
fn bonus_arg() -> Option<f32> {
    env::args().nth(2).and_then(|s| s.parse().ok())
}

fn main() {
    let games: usize = env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(50);
    let mut randy = Stats::default();
    let mut loonn = Stats::default();
    for g in 0..games {
        run_game(&mut randy, &mut loonn);
        eprint!("\rgame {}/{}", g + 1, games);
    }
    eprintln!();
    let rows: [(&str, fn(&Stats) -> f32); 18] = [
        ("final score", |s| s.score),
        ("  bird card points", |s| s.bird_points),
        ("  tuck/cache points", |s| s.other_turn_points),
        ("  end-of-round goals", |s| s.goal_points),
        ("  eggs", |s| s.egg_points),
        ("  bonus cards", |s| s.bonus_points),
        ("birds on mat", |s| s.birds),
        ("food left at end", |s| s.food_left),
        ("cards left in hand", |s| s.hand_left),
        ("turns: play bird", |s| s.turns_play),
        ("turns: lay eggs", |s| s.turns_eggs),
        ("turns: gain food", |s| s.turns_food),
        ("turns: draw cards", |s| s.turns_draw),
        ("turns: other", |s| s.turns_other),
        ("first bird on turn", |s| s.first_bird_turn),
        ("starting food kept", |s| s.initial_food),
        ("starting cards kept", |s| s.initial_hand),
        ("player-games", |s| s.players * s.players),
    ];
    println!("LooNN bird bonus: {}", bonus_arg().map_or("current".to_string(), |b| b.to_string()));
    println!("{:<24}{:>10}{:>10}", "per player-game", "Randy", "LooNN");
    for (name, f) in rows {
        let r = if name == "player-games" { randy.players } else { f(&randy) / randy.players };
        let l = if name == "player-games" { loonn.players } else { f(&loonn) / loonn.players };
        println!("{:<24}{:>10.1}{:>10.1}", name, r, l);
    }
}
