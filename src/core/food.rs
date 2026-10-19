use std::fmt::Display;
use std::ops::{ Add, AddAssign, Sub, SubAssign };

/// Represents a food token
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Food {
    Invertebrate,
    Seed,
    Fish,
    Fruit,
    Rodent,
    Wild,
}

impl From<FoodCollection> for Food {
    fn from(food_collection: FoodCollection) -> Self {
        if food_collection.invertebrates > 0 { return Self::Invertebrate }
        if food_collection.seeds > 0 { return Self::Seed }
        if food_collection.fish > 0 { return Self::Fish }
        if food_collection.fruits > 0 { return Self::Fruit }
        if food_collection.rodents > 0 { return Self::Rodent }
        panic!("Given FoodCollection did not contain any food");
    }
}

/// All of the different types of food
pub const FOODS: [Food; 5] = [
    Food::Invertebrate,
    Food::Seed,
    Food::Fish,
    Food::Fruit,
    Food::Rodent,
];

impl Display for Food {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Food::Invertebrate => write!(f, "an invertebrate"),
            Food::Seed => write!(f, "a seed"),
            Food::Fish => write!(f, "a fish"),
            Food::Fruit => write!(f, "a fruit"),
            Food::Rodent => write!(f, "a rodent"),
            Food::Wild => write!(f, "a wild"),
        }
    }
}


/// Represents a collection of food tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoodCollection {
    pub invertebrates: u8,
    pub seeds: u8,
    pub fish: u8,
    pub fruits: u8,
    pub rodents: u8,
    pub wilds: u8,
}

impl FoodCollection {
    pub const fn new() -> Self {
        Self { 
            invertebrates: 0, 
            seeds: 0, 
            fish: 0, 
            fruits: 0, 
            rodents: 0,
            wilds: 0,
        }
    }

    pub const fn all() -> Self {
        Self { 
            invertebrates: 1, 
            seeds: 1, 
            fish: 1, 
            fruits: 1, 
            rodents: 1,
            wilds: 0,
        }
    }

    pub fn contains(&self, food: Self) -> bool {
        let deficit = food.invertebrates.saturating_sub(self.invertebrates)
            + food.seeds.saturating_sub(self.seeds)
            + food.fish.saturating_sub(self.fish)
            + food.fruits.saturating_sub(self.fruits)
            + food.rodents.saturating_sub(self.rodents);
        let surplus = self.invertebrates.saturating_sub(food.invertebrates)
            + self.seeds.saturating_sub(food.seeds)
            + self.fish.saturating_sub(food.fish)
            + self.fruits.saturating_sub(food.fruits)
            + self.rodents.saturating_sub(food.rodents);
        if deficit > self.wilds { return false; }
        let remaining_wilds = self.wilds - deficit;
        remaining_wilds + surplus >= food.wilds 
    }
}

impl From<Food> for FoodCollection {
    fn from(food: Food) -> Self {
        let mut collection: FoodCollection = Self::new();
        collection += food;
        collection
    }
}

impl Add for FoodCollection {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            invertebrates: self.invertebrates.saturating_add(rhs.invertebrates),
            seeds: self.seeds.saturating_add(rhs.seeds),
            fish: self.fish.saturating_add(rhs.fish),
            fruits: self.fruits.saturating_add(rhs.fruits),
            rodents: self.rodents.saturating_add(rhs.rodents),
            wilds: self.wilds.saturating_add(rhs.wilds),
        }
    }
}

impl AddAssign for FoodCollection {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for FoodCollection {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            invertebrates: self.invertebrates.saturating_sub(rhs.invertebrates),
            seeds: self.seeds.saturating_sub(rhs.seeds),
            fish: self.fish.saturating_sub(rhs.fish),
            fruits: self.fruits.saturating_sub(rhs.fruits),
            rodents: self.rodents.saturating_sub(rhs.rodents),
            wilds: self.wilds.saturating_sub(rhs.wilds),
        }
    }
}

impl SubAssign for FoodCollection {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}


impl Add<Food> for FoodCollection {
    type Output = Self;
    fn add(mut self, rhs: Food) -> Self::Output {
        match rhs {
            Food::Invertebrate => self.invertebrates = self.invertebrates.saturating_add(1),
            Food::Seed => self.seeds = self.seeds.saturating_add(1),
            Food::Fish => self.fish = self.fish.saturating_add(1),
            Food::Fruit => self.fruits = self.fruits.saturating_add(1),
            Food::Rodent => self.rodents = self.rodents.saturating_add(1),
            Food::Wild => self.wilds = self.wilds.saturating_add(1),
        }
        self
    }
}

impl AddAssign<Food> for FoodCollection {
    fn add_assign(&mut self, rhs: Food) {
        *self = *self + rhs;
    }
}

impl Sub<Food> for FoodCollection {
    type Output = Self;

    fn sub(mut self, rhs: Food) -> Self::Output {
        match rhs {
            Food::Invertebrate => self.invertebrates = self.invertebrates.saturating_sub(1),
            Food::Seed => self.seeds = self.seeds.saturating_sub(1),
            Food::Fish => self.fish = self.fish.saturating_sub(1),
            Food::Fruit => self.fruits = self.fruits.saturating_sub(1),
            Food::Rodent => self.rodents = self.rodents.saturating_sub(1),
            Food::Wild => self.wilds = self.wilds.saturating_sub(1),
        }
        self
    }
}

impl SubAssign<Food> for FoodCollection {
    fn sub_assign(&mut self, rhs: Food) {
        *self = *self - rhs;
    }
}


/// Represents the food cost of a bird
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cost {
    /// Pay all of the given food
    All(FoodCollection),
    /// Pay 1 of the foods marked in the given collection
    OneOf(FoodCollection),
}

impl Cost {
    /// Returns each collection of food that would pay this cost
    pub fn payment_options(&self) -> Vec<FoodCollection> {
        match self {
            Cost::All(food) => vec![*food],
            Cost::OneOf(foods) => FOODS.into_iter()
                .filter(|&f| foods.contains(f.into()))
                .map(|f| f.into())
                .collect(),
        }
    }

    /// Returns every food that appears in this cost
    pub fn foods(&self) -> FoodCollection {
        match self { Cost::All(food) | Cost::OneOf(food) => *food }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_keeps_wilds_separate_from_rodents() {
        let a = FoodCollection { rodents: 2, wilds: 1, ..FoodCollection::new() };
        let b = FoodCollection { fish: 1, wilds: 3, ..FoodCollection::new() };
        let sum = a + b;
        assert_eq!(sum, FoodCollection {
            fish: 1, rodents: 2, wilds: 4, ..FoodCollection::new()
        });
    }
}
