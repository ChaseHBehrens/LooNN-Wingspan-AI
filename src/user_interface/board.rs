//! Prints LooNN's board between prompts

use crate::core::{ Habitat, Player, HABITATS };
use crate::describe::food_list;
use super::{ BOLD, DIM, RESET };

fn habitat_color(habitat: Habitat) -> &'static str {
    match habitat {
        Habitat::Forest => "\x1b[32m",
        Habitat::Grassland => "\x1b[33m",
        Habitat::Wetland => "\x1b[36m",
    }
}

/// A heading with one item per line below it
fn section(title: &str, items: Vec<String>, empty: &str) {
    println!("\n  {title}{RESET}");
    if items.is_empty() { println!("    {DIM}{empty}{RESET}"); }
    for item in items { println!("    {item}"); }
}

/// Each food held with how many, like "fish: 3"
fn food_counts(player: &Player) -> Vec<String> {
    let mut counts = Vec::<(&str, usize)>::new();
    for food in food_list(&player.food()) {
        match counts.last_mut() {
            Some((name, n)) if *name == food => *n += 1,
            _ => counts.push((food, 1)),
        }
    }
    counts.into_iter().map(|(food, n)| format!("{food}: {n}")).collect()
}

pub fn print(player: &Player, predicted: Option<f32>) {
    let predicted = predicted.map(|p| format!(" · predicted final {p:.1}")).unwrap_or_default();
    println!(
        "\n{BOLD}LooNN{RESET}  score {BOLD}{}{RESET}{predicted} · {} turns left",
        player.accumulated_score(), player.total_remaining_turns(),
    );
    for h in HABITATS {
        let birds = player.habitat(h).iter().map(|s| {
            let eggs = "●".repeat(s.eggs() as usize) + &"○".repeat(s.bird.egg_limit.saturating_sub(s.eggs()) as usize);
            format!("{} {DIM}{}pt{RESET} {eggs}", s.bird.name, s.bird.points)
        }).collect();
        let name = h.to_string();
        section(&format!("{}{}{}", habitat_color(h), name[..1].to_uppercase(), &name[1..]), birds, "empty");
    }
    section(&format!("{BOLD}Food"), food_counts(player), "none");
    section(&format!("{BOLD}Hand"), player.hand().iter().map(|b| format!("{} {DIM}{}pt{RESET}", b.name, b.points)).collect(), "none");
    section(&format!("{BOLD}Bonus"), player.bonus_cards().iter().map(|b| format!("{} {DIM}({} now){RESET}", b.name, b.score(player))).collect(), "none");
    let goals = player.goals().iter().enumerate()
        .map(|(i, g)| format!("Round {}: {} {DIM}({} now){RESET}", i + 1, g.name, g.score(player)))
        .collect();
    section(&format!("{BOLD}Goals"), goals, "none");
}
