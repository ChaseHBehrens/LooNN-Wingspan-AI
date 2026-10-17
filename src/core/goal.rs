use std::fmt::Display;
use super::Player;


/// Represents and end of round goal
pub struct Goal {
    pub name: &'static str,
    pub score: fn(&Player) -> usize,
}

impl Goal {
    /// Returns the score bonus from the goal
    pub fn score(&self, player: &Player) -> usize {
        (self.score)(player)
    }
}

impl Display for Goal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}


pub trait GoalDealer {
    /// Sets the end of round goals for the game
    fn set_goals(&mut self) -> [&'static Goal; 4];
}
