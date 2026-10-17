use crate::core::{ BonusCard, Player, Nest };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.bird.nest == Nest::Ground)
        .count()
    {
        0..=3 => 0,
        4..=5 => 4,
        6..   => 7,
    }
}

pub const ENCLOSURE_BUILDER: BonusCard = BonusCard {
    name: "Enclosure Builder",
    score: score,
};

