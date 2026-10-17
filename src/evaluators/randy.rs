use rand::{ rng, RngExt, rngs::ThreadRng };
use crate::core::{ Player, Evaluator };

/// Evaluates a player randomly
pub struct Randy {
    rng: ThreadRng,
}

impl Randy {
    pub fn new() -> Self { Self { rng: rng() } }
}

impl Evaluator for Randy {
    fn evaluate(&mut self, _player: &Player) -> f32 {
        self.rng.random_range(0.0..=500.0)
    }

    fn choose<T>(
        &mut self,
        options: impl Iterator<Item = (T, Player)>,
        _played: bool,
    ) -> Option<(T, crate::core::Player)> {
        options
            .map(|(t, p)| (self.evaluate(&p), t, p))
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, t, p)| (t, p))
    }
}

