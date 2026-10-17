use crate::core::{ Player, BonusCard };

fn score(player: &Player) -> usize {
    match player.bird_slots()
        .filter(|s| s.bird.cost.foods().fruits > 0)
        .count() 
    {
        0..=1 => 0,
        2..=3 => 3,
        4..   => 7,
    }
}

pub const VITICULTURIST: BonusCard = BonusCard {
    name: "Viticulturalist",
    score: score,
};

