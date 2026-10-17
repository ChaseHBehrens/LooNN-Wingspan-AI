use crate::core::{ Player, BonusCard };

fn score(player: &Player) -> usize {
    player.bird_slots()
        .filter(|s| s.bird.cost.foods().rodents > 0)
        .count() * 2
}

pub const RODENTOLOGIST: BonusCard = BonusCard {
    name: "Rodentologist",
    score: score,
};

