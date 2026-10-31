//! Terminal front end for a real game of Wingspan with LooNN in one seat
//! (src/bin/terminal.rs). You enter the cards and dice that come out, LooNN
//! prints its moves, and you make them on the table for it. The game itself is
//! `session::Session`, shared with the play page.

mod board;

use inquire::{ Confirm, InquireError, MultiSelect, Select, Text };
use inquire::validator::Validation;
use crate::birds::BIRDS;
use crate::bonuses::BONUSES;
use crate::core::{ Evaluator, HABITATS };
use crate::goals::GOALS;
use crate::session::{ has_between_turns_power, uses_fewest_birds, Prompt, PromptKind, Session };

pub use inquire::error::InquireResult;

const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
/// Rows shown at once in the long card lists
const PAGE_SIZE: usize = 10;

/// Plays games until the player quits, returning `Ok` when they choose to stop
/// and an error if they cancel a prompt (Esc or Ctrl+C)
pub fn play<E: Evaluator>(session: &mut Session<E>) -> InquireResult<()> {
    let mut shown = 0;
    loop {
        let log = session.log();
        if log.len() > shown {
            println!("\n{BOLD}LooNN's moves — make these on the table:{RESET}");
            for entry in &log[shown..] {
                println!("  {BOLD}{}{RESET}", entry.title);
                if entry.lines.is_empty() { println!("    {DIM}nothing to do{RESET}"); }
                for line in &entry.lines { println!("    • {line}"); }
            }
            shown = log.len();
        }

        let prompt = session.prompt().clone();
        if matches!(prompt.kind, PromptKind::Turn { .. } | PromptKind::Over) {
            let predicted = session.predicted();
            if let Some(player) = session.player() { board::print(player, predicted); }
        }
        if let PromptKind::Over = prompt.kind {
            if !Confirm::new("Play another game?").with_default(false).prompt()? { return Ok(()); }
            session.restart(rand::random());
            shown = 0;
            continue;
        }

        println!("\n{BOLD}{}{RESET}", prompt.title);
        if let Some(error) = &prompt.error { println!("{RED}{error}{RESET}"); }
        let seq = session.seq();
        let answer = ask(&prompt, session)?;
        println!("{DIM}LooNN is thinking…{RESET}");
        session.answer(seq, answer);
    }
}

/// Asks the prompt's question, returning the answer lines `Session::answer` expects
fn ask<E: Evaluator>(prompt: &Prompt, session: &Session<E>) -> InquireResult<Vec<String>> {
    let bird_names = || BIRDS.iter().map(|b| b.name).collect::<Vec<_>>();
    match &prompt.kind {
        PromptKind::Setup => ask_setup(),
        PromptKind::Turn { last } => ask_turn(session, *last),
        PromptKind::Dice { min, max, prefill } => ask_dice(*min, *max, prefill),
        PromptKind::Tray { prefill } => {
            let names = bird_names();
            let defaults: Vec<usize> = prefill.iter()
                .filter_map(|b| names.iter().position(|n| *n == b.name))
                .collect();
            let tray = MultiSelect::new("Face-up birds:", names)
                .with_default(&defaults)
                .with_page_size(PAGE_SIZE)
                .with_help_message("type to search, space to select up to 3, enter when done")
                .with_validator(|chosen: &[inquire::list_option::ListOption<&&str>]| Ok(if chosen.len() <= 3 {
                    Validation::Valid
                } else {
                    Validation::Invalid("The tray holds at most 3 birds".into())
                }))
                .prompt()?;
            Ok(tray.into_iter().map(String::from).collect())
        }
        PromptKind::Bird => {
            let bird = Select::new("Bird:", bird_names()).with_page_size(PAGE_SIZE).with_help_message("type to search").prompt()?;
            Ok(vec![bird.to_string()])
        }
        PromptKind::Bonus => {
            let names: Vec<&str> = BONUSES.iter().map(|b| b.name).collect();
            Ok(vec![Select::new("Bonus card:", names).with_page_size(PAGE_SIZE).prompt()?.to_string()])
        }
        PromptKind::Over => Ok(Vec::new()),
    }
}

fn ask_setup() -> InquireResult<Vec<String>> {
    let mut answer = Vec::new();
    println!("{DIM}Deal LooNN 5 birds and 2 bonus cards. It will choose what to keep, and gets 5 food minus the birds it keeps.{RESET}");
    let mut goals: Vec<&str> = GOALS.iter().map(|g| g.name).collect();
    for round in 1..=4 {
        let goal = Select::new(&format!("Round {round} goal:"), goals.clone()).with_page_size(PAGE_SIZE).prompt()?;
        goals.retain(|g| *g != goal);
        answer.push(format!("goal:{goal}"));
    }
    let mut birds: Vec<&str> = BIRDS.iter().map(|b| b.name).collect();
    for i in 1..=5 {
        let bird = Select::new(&format!("Bird dealt to LooNN ({i} of 5):"), birds.clone())
            .with_page_size(PAGE_SIZE)
            .with_help_message("type to search")
            .prompt()?;
        birds.retain(|b| *b != bird);
        answer.push(format!("bird:{bird}"));
    }
    let mut bonuses: Vec<&str> = BONUSES.iter().map(|b| b.name).collect();
    for i in 1..=2 {
        let bonus = Select::new(&format!("Bonus card dealt to LooNN ({i} of 2):"), bonuses.clone()).with_page_size(PAGE_SIZE).prompt()?;
        bonuses.retain(|b| *b != bonus);
        answer.push(format!("bonus:{bonus}"));
    }
    Ok(answer)
}

/// One checkbox before LooNN's turn: its label and the answer line it gives
struct Activity { label: String, line: String }

impl std::fmt::Display for Activity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.label) }
}

fn ask_turn<E: Evaluator>(session: &Session<E>, last: bool) -> InquireResult<Vec<String>> {
    let player = session.player().expect("a turn before setup");
    let mut options = Vec::<Activity>::new();
    let mut defaults = Vec::<usize>::new();
    let mut add = |label: String, line: String| options.push(Activity { label, line });
    if has_between_turns_power(player) {
        add("Another player laid eggs".into(), "lay_egg".into());
        add("Another player succeeded at a hunt".into(), "hunt".into());
        for h in HABITATS { add(format!("Another player played a bird in their {h}"), format!("play_bird:{h}")); }
        for food in ["invertebrate", "seed", "fish", "fruit", "rodent"] {
            add(format!("Another player gained {food}"), format!("gain_food:{food}"));
        }
    }
    let fewest: Vec<String> = HABITATS.into_iter().filter(|h| player.fewest_birds().contains(h)).map(|h| format!("fewest:{h}")).collect();
    if uses_fewest_birds(player) {
        for h in HABITATS {
            if player.fewest_birds().contains(&h) { defaults.push(options.len()); }
            options.push(Activity { label: format!("LooNN has the fewest birds in its {h} (ties count)"), line: format!("fewest:{h}") });
        }
    }
    options.push(Activity { label: "Another player used a power where all players benefit".into(), line: String::new() });

    if !last {
        println!("{DIM}When the other players have taken their turns, tick anything they did that LooNN's birds react to.{RESET}");
    }
    let chosen = MultiSelect::new("Since LooNN's last turn:", options)
        .with_default(&defaults)
        .with_help_message("space to tick, enter to let LooNN play")
        .prompt()?;
    let mut answer: Vec<String> = Vec::new();
    let mut shared = false;
    for activity in chosen {
        if activity.line.is_empty() { shared = true; } else { answer.push(activity.line); }
    }
    if !uses_fewest_birds(player) { answer.extend(fewest); }
    if shared {
        let names: Vec<&str> = BIRDS.iter().filter(|b| b.shared_power.is_some()).map(|b| b.name).collect();
        let bird = Select::new("Which bird's power?", names).with_page_size(PAGE_SIZE).prompt()?;
        answer.push(format!("shared:{bird}"));
    }
    Ok(answer)
}

/// Short names for typing dice
const DIE_ALIASES: [(&str, &str); 12] = [
    ("i", "invertebrate"), ("inv", "invertebrate"), ("invertebrate", "invertebrate"),
    ("s", "seed"), ("seed", "seed"),
    ("f", "fish"), ("fish", "fish"),
    ("fr", "fruit"), ("fruit", "fruit"),
    ("r", "rodent"), ("rodent", "rodent"),
    ("is", "invertebrate/seed"),
];

fn die_name(token: &str) -> Option<&'static str> {
    let token = token.to_lowercase();
    if token == "invertebrate/seed" || token == "i/s" { return Some("invertebrate/seed"); }
    DIE_ALIASES.iter().find(|(alias, _)| *alias == token).map(|(_, name)| *name)
}

fn short_die(name: &str) -> &'static str {
    match name { "invertebrate" => "i", "seed" => "s", "fish" => "f", "fruit" => "fr", "rodent" => "r", _ => "is" }
}

fn ask_dice(min: u8, max: u8, prefill: &[&'static str]) -> InquireResult<Vec<String>> {
    let initial: Vec<&str> = prefill.iter().map(|d| short_die(d)).collect();
    let initial = initial.join(" ");
    let count = if min == max { format!("exactly {max}") } else { format!("up to {max}") };
    let help = format!("{count} dice, separated by spaces: i=invertebrate s=seed f=fish fr=fruit r=rodent is=invertebrate/seed");
    let text = Text::new("Dice:")
        .with_initial_value(&initial)
        .with_help_message(&help)
        .with_validator(move |input: &str| {
            let tokens: Vec<&str> = input.split_whitespace().collect();
            if let Some(bad) = tokens.iter().find(|t| die_name(t).is_none()) {
                return Ok(Validation::Invalid(format!("\"{bad}\" is not a die face").into()));
            }
            let n = tokens.len() as u8;
            Ok(if n < min || n > max {
                Validation::Invalid(format!("Enter {} dice", if min == max { max.to_string() } else { format!("{min} to {max}") }).into())
            } else {
                Validation::Valid
            })
        })
        .prompt()?;
    Ok(text.split_whitespace().filter_map(die_name).map(String::from).collect())
}

/// True for the errors inquire gives when the player presses Esc or Ctrl+C
pub fn cancelled(error: &InquireError) -> bool {
    matches!(error, InquireError::OperationCanceled | InquireError::OperationInterrupted)
}
