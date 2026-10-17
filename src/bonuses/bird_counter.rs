use crate::core::{ Player, BonusCard, Niche };

fn score(player: &Player) -> usize {
    player.bird_slots()
        .filter(|s| s.bird.niche == Niche::Flocking)
        .count() * 2
}

pub const BIRD_COUNTER: BonusCard = BonusCard {
    name: "Bird Counter",
    score: score,
};

