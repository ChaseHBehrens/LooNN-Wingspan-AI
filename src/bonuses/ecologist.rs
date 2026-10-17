use crate::core::{ BonusCard, Player, HABITATS };

fn score(player: &Player) -> usize {
    HABITATS.into_iter()
        .map(|h| player.habitat(h).len())
        .min()
        .unwrap_or(0) * 2
}

pub const ECOLOGIST: BonusCard = BonusCard {
    name: "Ecologist",
    score: score,
};

