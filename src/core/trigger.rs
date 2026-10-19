use std::ops::AddAssign;
use super::{ 
    Habitat, HabitatCollection, 
    Food, FoodCollection, 
    Power, 
    Action,
};

/// Represents a trigger for a power to activate
#[derive(PartialEq, Eq)]
pub enum Trigger {
    LayEgg,
    PlayBird(Habitat),
    GainFood(Food),
    HuntSuccess,
}


/// Represents a set of triggers for a power to activate
#[derive(Clone)]
pub struct Triggers {
    /// lay eggs action occurred
    pub lay_egg: bool,
    /// play bird action occurred
    pub play_bird: HabitatCollection,
    /// gain food action occurred
    pub gain_food: FoodCollection,
    /// a hunt was successful
    pub hunt_success: bool,
    /// fewest birds in habitat
    pub fewest_birds: HabitatCollection,
    /// set of powers from opposing birds
    pub opposing_bird: Vec<Power>,
}

impl Triggers {
    pub fn new() -> Self {
        Self {
            lay_egg: false,
            play_bird: HabitatCollection { 
                forest: false, 
                grassland: false, 
                wetland: false,
            },
            gain_food: FoodCollection { 
                invertebrates: 0, 
                seeds: 0, 
                fish: 0, 
                fruits: 0, 
                rodents: 0, 
                wilds: 0,
            },
            hunt_success: false,
            fewest_birds: HabitatCollection::new(),
            opposing_bird: Vec::new(),
        }
    }
}

impl AddAssign for Triggers {
    fn add_assign(&mut self, rhs: Self) {
        self.lay_egg |= rhs.lay_egg;
        if rhs.play_bird.forest { self.play_bird.forest = true; }
        if rhs.play_bird.grassland { self.play_bird.grassland = true; }
        if rhs.play_bird.wetland { self.play_bird.wetland = true; }
        self.gain_food += rhs.gain_food;
        self.hunt_success |= rhs.hunt_success;
        if rhs.fewest_birds.forest { self.fewest_birds.forest = true; }
        if rhs.fewest_birds.grassland { self.fewest_birds.grassland = true; }
        if rhs.fewest_birds.wetland { self.fewest_birds.wetland = true; }
        self.opposing_bird.extend(rhs.opposing_bird);
    }
}

impl From<Vec<Action>> for Triggers {
    fn from(actions: Vec<Action>) -> Self {
        let mut triggers = Triggers::new();
        for action in actions {
            match action {
                Action::Activate(bird) => if let Some(power) = bird.shared_power {
                    triggers.opposing_bird.push(power)
                },
                Action::PlayBird(play_bird) => triggers.play_bird.insert(play_bird.habitat),
                Action::HuntSuccess => triggers.hunt_success = true,
                Action::InitialGainFood(food) => triggers.gain_food += food,
                Action::InitialLayEgg(_) => triggers.lay_egg = true,
                _ => continue,
            }
        }
        triggers
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rodent_gain_trigger_does_not_match_fish() {
        let mut triggers = Triggers::new();
        triggers += Triggers::from(vec![Action::InitialGainFood(Food::Rodent)]);
        assert_eq!(triggers.gain_food.wilds, 0);
        assert!(triggers.gain_food.contains(Food::Rodent.into()));
        assert!(!triggers.gain_food.contains(Food::Fish.into()));
    }
}
