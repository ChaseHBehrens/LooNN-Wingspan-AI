use std::fmt::Display;
use rand::RngExt;
use super::Food;


/// Represents a die
pub enum Die {
    Invertebrate,
    Seed,
    Fish,
    Fruit,
    Rodent,
    InvertebrateOrSeed,
}

impl Display for Die {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Die::Invertebrate => write!(f, "Invertebrate"),
            Die::Seed => write!(f, "Seed"),
            Die::Fish => write!(f, "Fish"),
            Die::Fruit => write!(f, "Fruit"),
            Die::Rodent => write!(f, "Rodent"),
            Die::InvertebrateOrSeed => write!(f, "Invertebrate/Seed"),
        }
    }
}


/// Represents a collection of dice
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Dice {
    pub invertebrates: u8,
    pub seeds: u8,
    pub fish: u8,
    pub fruits: u8,
    pub rodents: u8,
    pub invertebrate_or_seeds: u8,
}

impl Dice {
    pub fn new() -> Self {
        Self { 
            invertebrates: 0,
            seeds: 0,
            fish: 0,
            fruits: 0,
            rodents: 0,
            invertebrate_or_seeds: 0,
        }
    }

    pub fn random(rng: &mut impl RngExt, n: u8) -> Self {
        let mut dice = Dice::default();
        for _ in 0..n {
            match rng.random_range(0..6) {
                0 => dice.invertebrates += 1,
                1 => dice.seeds += 1,
                2 => dice.fish += 1,
                3 => dice.fruits += 1,
                4 => dice.rodents += 1,
                _ => dice.invertebrate_or_seeds += 1,
            }
        }
        dice
    }

    pub fn insert(&mut self, die: Die) {
        match die {
            Die::Invertebrate => self.invertebrates += 1,
            Die::Seed => self.seeds += 1,
            Die::Fish => self.fish += 1,
            Die::Fruit => self.fruits += 1,
            Die::Rodent => self.rodents += 1,
            Die::InvertebrateOrSeed => self.invertebrate_or_seeds += 1,
        }
    }
    
    pub fn remove(&mut self, die: Die) {
        match die {
            Die::Invertebrate => self.invertebrates -= 1,
            Die::Seed => self.seeds -= 1,
            Die::Fish => self.fish -= 1,
            Die::Fruit => self.fruits -= 1,
            Die::Rodent => self.rodents -= 1,
            Die::InvertebrateOrSeed => self.invertebrate_or_seeds -= 1,
        }
    }

    pub fn contains(&self, food: Food) -> bool {
        match food {
            Food::Invertebrate => (self.invertebrates > 0) || (self.invertebrate_or_seeds > 0),
            Food::Seed => (self.seeds > 0) || (self.invertebrate_or_seeds > 0),
            Food::Fish => self.fish > 0,
            Food::Fruit => self.fruits > 0,
            Food::Rodent => self.rodents > 0,
            Food::Wild => true,
        }
    }

    pub fn die_from_food(&self, food: Food) -> Die {
        match food {
            Food::Invertebrate => if self.invertebrates > 0 { 
                Die::Invertebrate
            } else {
                Die::InvertebrateOrSeed
            },
            Food::Seed => if self.seeds > 0 {
                Die::Seed
            } else {
                Die::InvertebrateOrSeed
            },
            Food::Fish => Die::Fish,
            Food::Fruit => Die::Fruit,
            Food::Rodent => Die::Rodent,
            _ => unimplemented!()
        }
    }
}

impl std::fmt::Display for Dice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut items = Vec::new();
        if self.invertebrates > 0 {
            let name = if self.invertebrates == 1 { "invertebrate" } else { "invertebrates" };
            items.push(format!("{} {}", self.invertebrates, name));
        }
        if self.seeds > 0 {
            let name = if self.seeds == 1 { "seed" } else { "seeds" };
            items.push(format!("{} {}", self.seeds, name));
        }
        if self.fish > 0 {
            items.push(format!("{} fish", self.fish));
        }
        if self.fruits > 0 {
            let name = if self.fruits == 1 { "fruit" } else { "fruits" };
            items.push(format!("{} {}", self.fruits, name));
        }
        if self.rodents > 0 {
            let name = if self.rodents == 1 { "rodent" } else { "rodents" };
            items.push(format!("{} {}", self.rodents, name));
        }
        if self.invertebrate_or_seeds > 0 {
            let name = if self.invertebrate_or_seeds == 1 { "invertebrate or seed" } else { "invertebrates or seeds" };
            items.push(format!("{} {}", self.invertebrate_or_seeds, name));
        }
        write!(f, "{}", items.join(", "))
    }
}


/// Represents a bird feeder
pub trait Feeder {
    /// Returns the contents of the bird feeder
    fn contents(&self) -> Dice;
    
    /// Removes a die from the feeder
    fn take_die(&mut self, die: Die);
    
    /// Returns the result of rolling dice outside feeder
    fn hunt(&mut self) -> Dice;

    /// Rolls all dice in feeder
    fn randomize(&mut self);

    /// Returns true if randomization is possible
    fn can_randomize(&self) -> bool {
        let contents = self.contents();
        2 > (contents.invertebrates != 0) as u8 + 
            (contents.seeds != 0) as u8 +
            (contents.fish != 0) as u8 +
            (contents.fruits != 0) as u8 +
            (contents.rodents != 0) as u8 +
            (contents.invertebrate_or_seeds != 0) as u8 
    }
}

