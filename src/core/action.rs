use std::fmt::Display;
use super::{ Food, FoodCollection, Bird, Habitat, BonusCard };

/// Represents the a choices made by a player
pub enum Action {
    Activate(&'static Bird),
    DiscardEgg(&'static Bird),
    DiscardFood(Food),
    DiscardBird(&'static Bird),
    PlayBird(PlayBird),
    LayEgg(&'static Bird),
    DrawBird(&'static Bird),
    GainFood(Food),
    Migrate(Habitat),
    DrawBonusCard(&'static BonusCard),
    TuckOrCache(u8),
    HuntSuccess,
    InitialGainFood(Food),
    InitialLayEgg(&'static Bird),
}

#[derive(Clone)]
pub struct PlayBird {
    pub bird: &'static Bird,
    pub food: FoodCollection,
    pub eggs: Vec<&'static Bird>,
    pub habitat: Habitat,
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Activate(bird) => write!(f, "Activate {}", bird),
            Action::DiscardEgg(bird) => write!(f, "Discard egg from {}", bird),
            Action::DiscardFood(food) => write!(f, "Discard {}", food),
            Action::DiscardBird(bird) => write!(f, "Discard {} from hand", bird),
            Action::PlayBird(play_bird) => {
                write!(f, "Play {} into {}", play_bird.bird, play_bird.habitat)?;
                if play_bird.food.invertebrates == 1 { write!(f, "\nDiscard an invertebrate")?; }
                if play_bird.food.invertebrates > 1 {
                    write!(f, "\nDiscard {} invertebrates", play_bird.food.invertebrates)?;
                }
                if play_bird.food.seeds == 1 { write!(f, "\nDiscard a seed")?; }
                if play_bird.food.seeds > 1 {
                    write!(f, "\nDiscard {} seeds", play_bird.food.seeds)?;
                }
                if play_bird.food.fish == 1 { write!(f, "\nDiscard a fish")?; }
                if play_bird.food.fish > 1 {
                    write!(f, "\nDiscard {} fish", play_bird.food.fish)?;
                }
                if play_bird.food.fruits == 1 { write!(f, "\nDiscard a fruit")?; }
                if play_bird.food.fruits > 1 {
                    write!(f, "\nDiscard {} fruits", play_bird.food.fruits)?;
                }
                if play_bird.food.rodents == 1 { write!(f, "\nDiscard a rodent")?; }
                if play_bird.food.rodents > 1 {
                    write!(f, "\nDiscard {} rodents", play_bird.food.rodents)?;
                }
                Ok(())
            },
            Action::LayEgg(bird) => write!(f, "Lay an egg on {}", bird),
            Action::DrawBird(bird) => write!(f, "Draw {}", bird),
            Action::GainFood(food) => write!(f, "Gain {}", food),
            Action::Migrate(habitat) => write!(f, "Migrate active bird to {}", habitat),
            Action::DrawBonusCard(bonus_card) => write!(f, "Take bonus card {}", bonus_card),
            Action::TuckOrCache(n) => {
                if *n == 1 { write!(f, "Tuck or cache 1 time") } 
                else { write!(f, "Tuck or cache {} times", n) }
            },
            Action::HuntSuccess => write!(f, "Hunt succeeded"),
            Action::InitialGainFood(food) => write!(f, "Gain {}", food),
            Action::InitialLayEgg(bird) => write!(f, "Lay an egg on {}", bird),
        }
    }
}

