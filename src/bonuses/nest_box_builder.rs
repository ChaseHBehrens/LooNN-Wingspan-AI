use crate::core::{ BonusCard, Player, Nest };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.bird.nest == Nest::Cavity)
        .count()
    {
        0..=3 => 0,
        4..=5 => 4,
        6..   => 7,
    }
}

pub const NEST_BOX_BUILDER: BonusCard = BonusCard {
    name: "Nest Box Builder",
    score: score,
};

