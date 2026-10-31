//! Checks that the native network (src/evaluators/native.rs, run by the web page)
//! predicts what the PyTorch network does, on positions from simulated games.
//! Run after exporting weights with neural_network/export_weights.py.
//!
//! Usage: cargo run --release --example native_parity [games]

use std::sync::Arc;
use std::time::Instant;
use rand::rng;
use loonn::core::{ BirdDealer, Evaluator, GoalDealer, Player, ScoringEvent, Triggers };
use loonn::evaluators::{ LooNN, NativeLooNN, Network };
use loonn::simulation::{ SimulatedBirdDeck, SimulatedBonusCardDeck, SimulatedFeeder, SimulatedGoalDeck };

const WEIGHTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/web/model.bin");

fn main() {
    let games: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(2);
    let bytes = std::fs::read(WEIGHTS).expect("Run neural_network/export_weights.py first");
    let network = Arc::new(Network::from_bytes(&bytes).expect("Bad weights file"));
    let mut native = NativeLooNN::predictor(network.clone());
    let mut python = LooNN::with_bird_bonus(0.0).expect("Failed to construct LooNN");

    // Positions from games the native network plays against itself
    let mut positions = Vec::<Player>::new();
    for _ in 0..games {
        let mut birds = SimulatedBirdDeck::new(false);
        let mut bonuses = SimulatedBonusCardDeck::new(false);
        let mut feeder = SimulatedFeeder::new(false);
        let goals = SimulatedGoalDeck::new(false).set_goals();
        let mut chooser = NativeLooNN::new(network.clone());
        let mut player = Player::new(&mut chooser, goals, &mut birds, &mut bonuses, false);
        positions.push(player.clone());
        loop {
            birds.refill_face_up_birds();
            let result = player.play_turn(&mut chooser, &mut feeder, &mut birds, &mut bonuses, &Triggers::new(), &mut rng(), false);
            positions.push(player.clone());
            if let Err(ScoringEvent::EndOfGame) = result { break; }
        }
    }

    let start = Instant::now();
    let native_scores: Vec<f32> = positions.iter().map(|p| native.evaluate(p)).collect();
    let native_time = start.elapsed();
    let python_scores = python.evaluate_all(&positions);
    let worst = native_scores.iter().zip(&python_scores).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
    for (n, p) in native_scores.iter().zip(&python_scores).step_by(9) { println!("native {n:8.3}   pytorch {p:8.3}"); }
    println!("{} positions, largest difference {worst:.5} points", positions.len());
    println!("native: {:.2} ms per position", native_time.as_secs_f64() * 1000.0 / positions.len() as f64);
    let start = Instant::now();
    for p in &positions { std::hint::black_box(loonn::evaluators::features::vectorize_player(p)); }
    println!("  of which vectorizing: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0 / positions.len() as f64);
}
