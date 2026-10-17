use crate::core::FoodCollection;

use super::{ Nest, Niche, Habitat, HabitatCollection };

/// Represents the power of a bird
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Power {
    /// plays a bird in the same habitat as this bird
    pub play_bird_in_this_habitat: bool,
    /// plays a bird from hand to a habitat included in the `HabitatCollection`
    pub play_bird: HabitatCollection,
    /// if the farthest bird to the right you may move it to another habitat
    pub migration: bool,
    /// successful hunt of `Bird` is required
    pub hunt_bird: Option<u16>,
    /// food drawn must be one of these options
    pub food_constraint: FoodCollection,
    /// successful hunt of `Food` is required
    pub hunt_dice: bool,
    /// drawing of `Food` will be from the feeder rather than supply
    pub feeder: bool,
    /// gains `u8` number of food
    pub gain_food: u8,
    /// discard one `Food`
    pub discard_food: bool,
    /// eggs can be laid on this bird
    pub self_constraint: bool,
    /// eggs can be laid on other birds of this nest type
    pub nest_constraint: Nest,
    /// lay one egg on all birds of `Nest` type
    pub lay_eggs_on_all: bool,
    /// lay an egg on this bird
    pub lay_eggs: u8,
    /// discard one egg
    pub discard_egg: bool,
    /// power is not possible if not the fewest birds in `Habitat`
    pub fewest_birds: Option<Habitat>,
    /// drawn cards can be from face up options
    pub face_up_constraint: bool,
    /// drawn cards can be from the deck
    pub deck_constraint: bool,
    /// draw `u8` number of bird cards
    pub draw_bird: u8,
    /// tucks or caches `u8` number of points
    pub tuck_or_cache: u8,
    /// discard one bird card
    pub discard_bird: bool,
    /// repeats the when activated power of another bird in this habitat
    pub repeat_power: Option<Niche>,
    /// draw two bonus cards and keep one
    pub draw_bonus: bool,
}

pub const NO_POWER: Power = Power {
    play_bird_in_this_habitat: false,
    play_bird: HabitatCollection::new(),
    migration: false,
    feeder: false,
    hunt_dice: false,
    hunt_bird: None,
    gain_food: 0,
    food_constraint: FoodCollection::new(),
    discard_food: false,
    self_constraint: false,
    nest_constraint: Nest::None,
    lay_eggs_on_all: false,
    lay_eggs: 0,
    discard_egg: false,
    fewest_birds: None,
    face_up_constraint: false,
    deck_constraint: false,
    draw_bird: 0,
    tuck_or_cache: 0,
    discard_bird: false,
    repeat_power: None,
    draw_bonus: false,
};

