use crate::core::{ Player, BonusCard };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.bird.cost.foods().fish > 0)
        .count() 
    {
        0..=1 => 0,
        2..=3 => 3,
        4..   => 8,
    }
}

pub const FISHERY_MANAGER: BonusCard = BonusCard {
    name: "Fishery Manager",
    score: score,
};

