use crate::core::{ Player, BonusCard };

fn score(player: &Player) -> usize {
    const COLORS: [&str; 24] = [
        "ash", "black", "blue", "bronze", "brown", 
        "cerulean", "chestnut", "ferruginous", "gold", 
        "gray", "green", "indigo", "lazuli", "purple", 
        "red", "rose", "roseate", "ruby", "ruddy", 
        "rufous", "snowy", "violet", "white", "yellow",
    ];
    // contain a color only as part of another word
    const EXCLUDED: [&str; 1] = ["Barred Owl"];
    let color_in_name = |name: &str| -> bool {
         !EXCLUDED.contains(&name) && COLORS.iter().any(|s| name.to_lowercase().contains(s))
    };
    match player.bird_slots()
        .filter(|s| color_in_name(s.bird.name))
        .count() 
    {
        0..=3 => 0,
        4..=5 => 3,
        6..   => 7,
    }
}

pub const PHOTOGRAPHER: BonusCard = BonusCard {
    name: "Photographer",
    score: score,
};

