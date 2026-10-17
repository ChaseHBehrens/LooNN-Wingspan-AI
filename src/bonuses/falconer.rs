use crate::core::{ Player, BonusCard, Niche };

fn score(player: &Player) -> usize {
    player.bird_slots()
        .filter(|s| s.bird.niche == Niche::Predator)
        .count() * 2
}

pub const FALCONER: BonusCard = BonusCard {
    name: "Falconer",
    score: score,
};

