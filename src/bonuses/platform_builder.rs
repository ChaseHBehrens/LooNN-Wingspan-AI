use crate::core::{ BonusCard, Player, Nest };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.bird.nest == Nest::Platform)
        .count()
    {
        0..=3 => 0,
        4..=5 => 4,
        6..   => 7,
    }
}

pub const PLATFORM_BUILDER: BonusCard = BonusCard {
    name: "Platform Builder",
    score: score,
};

