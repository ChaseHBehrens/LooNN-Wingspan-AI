use crate::core::{ BonusCard, Habitat, Player };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.bird.habitat == Habitat::Grassland.into())
        .count() 
    {
        0..=1 => 0,
        2..=3 => 3,
        4..   => 8,
    }
}

pub const PRAIRIE_MANAGER: BonusCard = BonusCard {
    name: "Prairie Manager",
    score: score,
};

