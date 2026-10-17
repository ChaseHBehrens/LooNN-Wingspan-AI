use crate::core::{ BonusCard, Cost, FoodCollection, Player };

fn score(player: &Player) -> usize {
    player.bird_slots()
        .filter(|s| match s.bird.cost {
            Cost::All(food) => food.invertebrates > 0 && food == FoodCollection { 
                invertebrates: food.invertebrates, 
                ..FoodCollection::new() 
            },
            Cost::OneOf(_) => false,
        })
        .count() * 2
}

pub const FOOD_WEB_EXPERT: BonusCard = BonusCard {
    name: "Food Web Expert",
    score: score,
};

