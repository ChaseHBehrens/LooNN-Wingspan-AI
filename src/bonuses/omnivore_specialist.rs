use crate::core::{ Player, BonusCard };

fn score(player: &Player) -> usize {
    player.bird_slots()
        .filter(|s| s.bird.cost.foods().wilds > 0)
        .count() * 2
}

pub const OMNIVORE_SPECIALIST: BonusCard = BonusCard {
    name: "Omnivore Specialist",
    score: score,
};

