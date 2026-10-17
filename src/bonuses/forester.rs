use crate::core::{ BonusCard, Habitat, Player };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.bird.habitat == Habitat::Forest.into())
        .count() 
    {
        0..=2 => 0,
        3..=4 => 4,
        5..   => 8,
    }
}

pub const FORESTER: BonusCard = BonusCard {
    name: "Forester",
    score: score,
};

