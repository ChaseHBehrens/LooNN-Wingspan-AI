use crate::core::{ Player, BonusCard };

fn score(player: &Player) -> usize {
    player.bird_slots()
        .filter(|s| s.bird.name.contains("'s"))
        .count() * 2
}

pub const HISTORIAN: BonusCard = BonusCard {
    name: "Historian",
    score: score,
};

