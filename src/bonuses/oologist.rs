use crate::core::{ BonusCard, Player };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.eggs() >= 1)
        .count()
    {
        0..=6 => 0,
        7..=8 => 3,
        9..   => 6,
    }
}

pub const OOLOGIST: BonusCard = BonusCard {
    name: "Oologist",
    score: score,
};

