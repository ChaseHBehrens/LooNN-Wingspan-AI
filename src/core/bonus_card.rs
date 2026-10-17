use std::fmt::Display;
use super::Player;

/// Represents a bonus card 
pub struct BonusCard {
    pub name: &'static str,
    pub score: fn(&Player) -> usize,
}

impl BonusCard {
    /// Returns the score bonus from the card
    pub fn score(&self, player: &Player) -> usize {
        (self.score)(player)
    }
}

impl Display for BonusCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}


/// Represents bonus card deck
pub trait BonusDealer {
    /// Draws a bonus card
    fn draw(&mut self) -> &'static BonusCard;

    /// Discards a bonus card
    fn discard(&mut self, bonus: &'static BonusCard);
}

