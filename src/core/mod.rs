pub mod bird; pub use bird::*;
pub mod bonus_card; pub use bonus_card::*;
pub mod goal; pub use goal::*;
pub mod food; pub use food::*;
pub mod habitat; pub use habitat::*;
pub mod nest; pub use nest::*;
pub mod player; pub use player::*;
pub mod power; pub use power::*;
pub mod trigger; pub use trigger::*;
pub mod feeder; pub use feeder::*;
pub mod action; pub use action::*;
pub mod niche; pub use niche::*;

/// Evaluation fn for a player
pub trait Evaluator { 
    /// Returns the predicted end of game score for the given player
    fn evaluate(&mut self, player: &Player) -> f32; 

    /// Returns the predicted end of game score for each of the given players
    fn evaluate_all(&mut self, players: &[Player]) -> Vec<f32> {
        players.iter().map(|p| self.evaluate(p)).collect()
    }
    
    /// Chooses the predicted highest scoring player
    fn choose<T>(
        &mut self,
        options: impl Iterator<Item = (T, Player)>,
        played: bool,
    ) -> Option<(T, Player)>;
}

