use crate::core::{ Player, BonusCard };

fn score(player: &Player) -> usize {
    match player.hand().len() {
        0..=4 => 0,
        5..=7 => 4,
        8..   => 7,
    }
}

pub const VISIONARY_LEADER: BonusCard = BonusCard {
    name: "Visionary Leader",
    score: score,
};

