//! Play a real game of Wingspan with LooNN in one seat, from the terminal. The
//! same game as the play page: you enter the cards and dice that come out, LooNN
//! prints its moves, and you make them on the table for it.
//!
//! Usage: cargo run --release --bin terminal [--pytorch]
//!
//! LooNN uses the network exported to web/model.bin (run
//! neural_network/export_weights.py after training), built into the binary so
//! release downloads run on their own. --pytorch uses the PyTorch checkpoint
//! directly instead, which is always current but much slower; it needs the
//! `python` feature, which release downloads leave out.

use std::sync::Arc;
use loonn::core::Evaluator;
use loonn::evaluators::{ NativeLooNN, Network };
use loonn::session::Session;
use loonn::user_interface::{ cancelled, play };

const WEIGHTS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/web/model.bin"));

fn run<E: Evaluator>(chooser: E, predictor: E) {
    println!("Loading LooNN…");
    let mut session = Session::new(chooser, predictor, rand::random());
    match play(&mut session) {
        Ok(()) => {}
        Err(e) if cancelled(&e) => println!("Stopped."),
        Err(e) => eprintln!("Terminal error: {e}"),
    }
}

fn main() {
    if std::env::args().any(|a| a == "--pytorch") {
        #[cfg(feature = "python")]
        {
            use loonn::evaluators::LooNN;
            let chooser = LooNN::new().expect("Failed to construct LooNN");
            // Predicts LooNN's final score without the bonus it adds per bird when choosing
            let predictor = LooNN::with_bird_bonus(0.0).expect("Failed to construct LooNN");
            return run(chooser, predictor);
        }
        #[cfg(not(feature = "python"))]
        {
            eprintln!("This build doesn't include PyTorch support; run from source to use --pytorch.");
            std::process::exit(1);
        }
    }
    let network = Arc::new(Network::from_bytes(WEIGHTS).expect("Bad weights file"));
    run(NativeLooNN::new(network.clone()), NativeLooNN::predictor(network));
}
