use crate::core::{ BonusCard, Player };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.bird.wingspan <= 30)
        .count()
    {
        0..=3 => 0,
        4..=5 => 3,
        6..   => 6,
    }
}

pub const PASSERINE_SPECIALIST: BonusCard = BonusCard {
    name: "Passerine Specialist",
    score: score,
};

