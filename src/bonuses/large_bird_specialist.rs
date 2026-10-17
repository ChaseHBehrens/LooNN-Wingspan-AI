use crate::core::{ BonusCard, Player };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.bird.wingspan > 65)
        .count()
    {
        0..=3 => 0,
        4..=5 => 3,
        6..   => 6,
    }
}

pub const LARGE_BIRD_SPECIALIST: BonusCard = BonusCard {
    name: "Large Bird Specialist",
    score: score,
};

