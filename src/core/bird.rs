use std::fmt::Display;
use super::{ Nest, Niche, HabitatCollection, Cost, Power, Trigger };

/// Represents a birds power
pub enum Activation {
    None,
    WhenActivated(Power),
    OnceBetweenTurns(Trigger, Power),
    WhenPlayed(Power),
}

/// Represents a bird card
pub struct Bird {
    pub name: &'static str,
    pub wingspan: u16,
    pub cost: Cost,
    pub points: u8,
    pub niche: Niche,
    pub habitat: HabitatCollection,
    pub nest: Nest,
    pub egg_limit: u8,
    pub power: Activation,
    pub shared_power: Option<Power>,
}

impl Display for Bird {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}


/// Represents the bird draw tray
pub trait BirdDealer {
    /// Returns all cards in the deck
    fn deck(&self) -> impl Iterator<Item = &'static Bird>;

    /// Draws a bird from the deck
    fn draw_from_deck(&mut self) -> &'static Bird;
    
    /// Draws a bird from face up options
    fn draw_face_up(&mut self, bird: &'static Bird);
    
    /// Refills the face up card pool
    fn refill_face_up_birds(&mut self);

    /// Returns the face up bird options
    fn face_up_birds(&mut self) -> &[&'static Bird];
    
    /// Discards a bird
    fn discard(&mut self, bird: &'static Bird);
}
