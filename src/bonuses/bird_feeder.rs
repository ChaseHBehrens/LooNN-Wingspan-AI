use crate::core::{ Player, BonusCard };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.bird.cost.foods().seeds > 0)
        .count() 
    {
        0..=4 => 0,
        5..=7 => 3,
        8..   => 7,
    }
}

pub const BIRD_FEEDER: BonusCard = BonusCard {
    name: "Bird Feeder",
    score: score,
};

