use crate::core::{ BonusCard, Habitat, Player };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.bird.habitat == Habitat::Wetland.into())
        .count() 
    {
        0..=2 => 0,
        3..=4 => 3,
        5..   => 7,
    }
}

pub const WETLAND_SPECIALIST: BonusCard = BonusCard {
    name: "Wetland Scientist",
    score: score,
};

