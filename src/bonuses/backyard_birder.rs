use crate::core::{ Player, BonusCard };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.bird.points < 4)
        .count() 
    {
        0..=4 => 0,
        5..=6 => 3,
        7..   => 6,
    }
}

pub const BACKYARD_BIRDER: BonusCard = BonusCard {
    name: "Backyard Birder",
    score: score,
};

