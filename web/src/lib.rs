//! The play page's game, compiled to WebAssembly so the page runs on its own
//! (on GitHub Pages) with no server. web/worker.js runs it off the page's thread.
//!
//! Build with web/build.sh.

use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use loonn::evaluators::{ NativeLooNN, Network };
use loonn::session::Session;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(message: &str);
}

thread_local! {
    /// The last panic's message, which the worker reports after the call traps
    static LAST_PANIC: RefCell<String> = const { RefCell::new(String::new()) };
}

/// A game with LooNN in one seat. Every method returns the state as JSON.
#[wasm_bindgen]
pub struct Game {
    session: Session<NativeLooNN>,
}

#[wasm_bindgen]
impl Game {
    /// `weights` is web/model.bin, from neural_network/export_weights.py
    #[wasm_bindgen(constructor)]
    pub fn new(weights: &[u8], seed: f64) -> Result<Game, JsError> {
        std::panic::set_hook(Box::new(|info| {
            let message = info.to_string();
            error(&message);
            LAST_PANIC.with(|p| *p.borrow_mut() = message);
        }));
        let network = Arc::new(Network::from_bytes(weights).map_err(|e| JsError::new(&e))?);
        let session = Session::new(NativeLooNN::new(network.clone()), NativeLooNN::predictor(network), seed as u64);
        Ok(Game { session })
    }

    pub fn state(&mut self) -> String { self.session.state_json() }

    /// Answers prompt `seq` with one value per line
    pub fn answer(&mut self, seq: f64, answer: &str) -> String {
        let lines = answer.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
        self.session.answer(seq as u64, lines);
        self.state()
    }

    pub fn restart(&mut self, seed: f64) -> String {
        self.session.restart(seed as u64);
        self.state()
    }
}

/// JSON for every card, so the page can offer and show them
#[wasm_bindgen]
pub fn catalog() -> String { loonn::session::catalog() }

#[wasm_bindgen]
pub fn last_panic() -> String { LAST_PANIC.with(|p| p.borrow().clone()) }
