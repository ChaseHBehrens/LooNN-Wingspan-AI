use crate::core::{ Player, BonusCard };

fn score(player: &Player) -> usize {
    player.bird_slots()
        .filter(|s| s.eggs() >= 4)
        .count()
}

pub const BREEDING_MANAGER: BonusCard = BonusCard {
    name: "Breeding Manager",
    score: score,
};

