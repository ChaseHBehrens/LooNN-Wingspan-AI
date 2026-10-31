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

fn list(items: Vec<String>) -> String {
    if items.is_empty() { format!("{DIM}none{RESET}") } else { items.join(", ") }
}

pub fn print(player: &Player, predicted: Option<f32>) {
    let predicted = predicted.map(|p| format!(" · predicted final {p:.1}")).unwrap_or_default();
    println!(
        "\n{BOLD}LooNN{RESET}  score {BOLD}{}{RESET}{predicted} · {} turns left",
        player.accumulated_score(), player.total_remaining_turns(),
    );
    for h in HABITATS {
        let birds: Vec<String> = player.habitat(h).iter().map(|s| {
            let eggs = "●".repeat(s.eggs() as usize) + &"○".repeat(s.bird.egg_limit.saturating_sub(s.eggs()) as usize);
            format!("{} {DIM}{}pt{RESET} {eggs}", s.bird.name, s.bird.points)
        }).collect();
        let name = h.to_string();
        println!("  {}{name:<10}{RESET}{}", habitat_color(h), if birds.is_empty() { format!("{DIM}empty{RESET}") } else { birds.join("  │  ") });
    }
    println!("  {BOLD}Food{RESET}      {}", list(food_list(&player.food()).into_iter().map(String::from).collect()));
    println!("  {BOLD}Hand{RESET}      {}", list(player.hand().iter().map(|b| format!("{} {DIM}{}pt{RESET}", b.name, b.points)).collect()));
    println!("  {BOLD}Bonus{RESET}     {}", list(player.bonus_cards().iter().map(|b| format!("{} {DIM}({} now){RESET}", b.name, b.score(player))).collect()));
    let goals: Vec<String> = player.goals().iter().enumerate()
        .map(|(i, g)| format!("R{} {} {DIM}({} now){RESET}", i + 1, g.name, g.score(player)))
        .collect();
    println!("  {BOLD}Goals{RESET}     {}", goals.join(" · "));
}
