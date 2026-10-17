use crate::core::{ Player, BonusCard };

fn score(player: &Player) -> usize {
    const GEOGRAPHY_TERMS: [&str; 16] = [
        "american", "atlantic", "baltimore", "california", 
        "canada", "carolina", "chihuahua", "eastern", "inca", 
        "mississippi", "mountain", "northern", "prairie", 
        "sandhill", "savannah", "western",
    ];
    let geography_term_in_name = |name: &str| -> bool {
         GEOGRAPHY_TERMS.iter().any(|s| name.to_lowercase().contains(s))
    };
    match player.bird_slots()
        .filter(|s| geography_term_in_name(s.bird.name))
        .count() 
    {
        0..=1 => 0,
        2..=3 => 3,
        4..   => 7,
    }
}

pub const CARTOGRAPHER: BonusCard = BonusCard {
    name: "Cartographer",
    score: score,
};

