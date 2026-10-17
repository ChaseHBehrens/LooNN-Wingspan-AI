use crate::core::{ Player, BonusCard };

fn score(player: &Player) -> usize {
    const BODY_PARTS: [&str; 21] = [
        "beak", "belly", "bellied", "bill", "breast", "cap", 
        "chin", "collar", "crest", "crown", "eye",
        "face", "head", "neck", "rump", "shoulder",
        "tail", "throat", "wing",
        // not printed on the card but counted officially (Canvasback, Tufted Titmouse)
        "back", "tuft",
    ];
    // contain a body part only as part of another word
    const EXCLUDED: [&str; 2] = ["Burrowing Owl", "Trumpeter Swan"];
    let body_part_in_name = |name: &str| -> bool {
         !EXCLUDED.contains(&name) && BODY_PARTS.iter().any(|s| name.to_lowercase().contains(s))
    };
    match player.bird_slots()
        .filter(|s| body_part_in_name(s.bird.name))
        .count() 
    {
        0..=1 => 0,
        2..=3 => 3,
        4..   => 7,
    }
}

pub const ANATOMIST: BonusCard = BonusCard {
    name: "Anatomist",
    score: score,
};

