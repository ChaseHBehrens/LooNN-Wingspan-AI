//! Text and JSON descriptions of cards, shared by the web tools in `src/bin/`
//! (`replay` for the game viewer and `play` for playing alongside LooNN)

use std::fmt::Write;
use crate::core::{
    Activation, Bird, Cost, FoodCollection, HabitatCollection, Nest, Niche, Power, Trigger, HABITATS,
};

pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => { let _ = write!(out, "\\u{:04x}", c as u32); }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn food_list(f: &FoodCollection) -> Vec<&'static str> {
    let mut v = Vec::new();
    for (n, name) in [
        (f.invertebrates, "invertebrate"), (f.seeds, "seed"), (f.fish, "fish"),
        (f.fruits, "fruit"), (f.rodents, "rodent"), (f.wilds, "wild"),
    ] {
        for _ in 0..n { v.push(name); }
    }
    v
}

fn food_options(f: &FoodCollection) -> String {
    let foods = food_list(f);
    if foods.is_empty() || foods.len() >= 5 { "food of any type".into() } else { foods.join(" or ") }
}

fn plural(n: u8, one: &str, many: &str) -> String {
    if n == 1 { format!("1 {one}") } else { format!("{n} {many}") }
}

/// Card-style text for a power, following the order `Player::run_power` applies its parts
fn describe_power(p: &Power) -> String {
    let mut conditions = Vec::<String>::new();
    let mut steps = Vec::<String>::new();
    if let Some(h) = p.fewest_birds {
        conditions.push(format!("If you have the fewest birds in your {h} (ties count):"));
    }
    if p.hunt_dice {
        conditions.push(format!("Roll the dice outside the feeder. If any show {}:", food_options(&p.food_constraint)));
    }
    if let Some(w) = p.hunt_bird {
        conditions.push(format!("Reveal a card from the deck. If its wingspan is under {w} cm:"));
    }
    if p.discard_egg { steps.push("discard 1 egg from any of your birds".into()); }
    if p.discard_food { steps.push(format!("discard 1 {}", food_options(&p.food_constraint))); }
    if p.discard_bird && p.tuck_or_cache > 0 {
        steps.push(format!("tuck 1 card from your hand behind this bird (+{})", plural(p.tuck_or_cache, "point", "points")));
    }
    if p.draw_bird > 0 {
        let from = match (p.deck_constraint, p.face_up_constraint) {
            (true, true) => "the tray or deck", (true, false) => "the deck", _ => "the tray",
        };
        steps.push(format!("draw {} from {from}", plural(p.draw_bird, "bird card", "bird cards")));
    }
    if p.discard_bird && p.tuck_or_cache == 0 { steps.push("discard 1 card from your hand".into()); }
    if p.lay_eggs > 0 || p.lay_eggs_on_all {
        let nest = match p.nest_constraint {
            Nest::Platform => Some("platform"), Nest::Bowl => Some("bowl"), Nest::Cavity => Some("cavity"),
            Nest::Ground => Some("ground"), Nest::Wild => Some("any"), Nest::None => None,
        };
        let target = match nest {
            None => "this bird".to_string(),
            Some("any") => format!("{} of your birds{}",
                if p.lay_eggs_on_all { "each" } else { "any" },
                if p.self_constraint { "" } else { " (not this one)" }),
            Some(n) => format!("{} bird with a {n} nest{}",
                if p.lay_eggs_on_all { "each" } else { "any" },
                if p.self_constraint { "" } else { " (not this one)" }),
        };
        if p.lay_eggs > 0 { steps.push(format!("lay {} on {target}", plural(p.lay_eggs, "egg", "eggs"))); }
        if p.lay_eggs_on_all { steps.push(format!("lay 1 egg on {target}")); }
    }
    if p.gain_food > 0 {
        let source = if p.feeder { "the feeder" } else { "the supply" };
        let options = food_options(&p.food_constraint);
        let what = if p.gain_food > 1 && options.contains(" or ") {
            format!("{} food ({options})", p.gain_food)
        } else { format!("{} {options}", p.gain_food) };
        steps.push(format!("gain {what} from {source}"));
    }
    if p.migration { steps.push("if this is the rightmost bird in its row, you may move it to another habitat".into()); }
    if p.play_bird != HabitatCollection::new() {
        let habs: Vec<String> = HABITATS.into_iter().filter(|h| p.play_bird.contains(h)).map(|h| h.to_string()).collect();
        steps.push(format!("play a bird in your {} (paying its costs)", habs.join(" or ")));
    }
    if p.play_bird_in_this_habitat { steps.push("play another bird in this habitat (paying its costs)".into()); }
    if p.tuck_or_cache > 0 {
        let pts = plural(p.tuck_or_cache, "point", "points");
        if !p.discard_bird { steps.push(if p.feeder { format!("cache 1 {} from the feeder on this bird (+{pts})", food_options(&p.food_constraint)) }
            else if p.hunt_bird.is_some() { format!("tuck it behind this bird (+{pts})") }
            else if p.hunt_dice { format!("cache 1 on this bird (+{pts})") }
            else { format!("tuck or cache (+{pts})") }); }
    }
    if let Some(niche) = p.repeat_power {
        let kind = match niche { Niche::Predator => "predator ", Niche::Flocking => "flocking ", _ => "" };
        steps.push(format!("repeat the brown power of another {kind}bird in this habitat"));
    }
    if p.draw_bonus { steps.push("draw 2 bonus cards and keep 1".into()); }
    if steps.is_empty() { steps.push("(no effect is defined for this power in the game data)".into()); }
    let mut text = steps.join(", then ");
    if let Some(first) = text.get(0..1) { text = first.to_uppercase() + &text[1..]; }
    if conditions.is_empty() { text } else { format!("{} {}", conditions.join(" "), text) }
}

fn trigger_text(t: &Trigger) -> String {
    match t {
        Trigger::LayEgg => "lays eggs".into(),
        Trigger::PlayBird(h) => format!("plays a bird in their {h}"),
        Trigger::GainFood(f) => format!("gains {f}"),
        Trigger::HuntSuccess => "succeeds at a hunt".into(),
    }
}

pub fn power_text(b: &Bird) -> String {
    let main = match &b.power {
        Activation::None => return String::new(),
        Activation::WhenActivated(p) => format!("WHEN ACTIVATED: {}", describe_power(p)),
        Activation::WhenPlayed(p) => format!("WHEN PLAYED: {}", describe_power(p)),
        Activation::OnceBetweenTurns(t, p) => format!(
            "ONCE BETWEEN TURNS: When another player {}, {}", trigger_text(t), {
                let d = describe_power(p);
                d.get(0..1).map(|c| c.to_lowercase() + &d[1..]).unwrap_or(d)
            }),
    };
    match &b.shared_power {
        Some(shared) => format!("{main}. All other players also: {}.", {
            let d = describe_power(shared);
            d.get(0..1).map(|c| c.to_lowercase() + &d[1..]).unwrap_or(d)
        }),
        None => format!("{main}."),
    }
}

pub fn bird_json(b: &Bird) -> String {
    let habitats: Vec<String> = HABITATS.into_iter()
        .filter(|h| b.habitat.contains(h))
        .map(|h| esc(&h.to_string()))
        .collect();
    let nest = match b.nest {
        Nest::Platform => "platform", Nest::Bowl => "bowl", Nest::Cavity => "cavity",
        Nest::Ground => "ground", Nest::Wild => "wild", Nest::None => "none",
    };
    let (cost_kind, cost_food) = match &b.cost {
        Cost::All(f) => ("all", food_list(f)),
        Cost::OneOf(f) => ("one_of", food_list(f)),
    };
    let power = match b.power {
        Activation::None => "none",
        Activation::WhenActivated(_) => "when_activated",
        Activation::OnceBetweenTurns(..) => "between_turns",
        Activation::WhenPlayed(_) => "when_played",
    };
    let cost_food: Vec<String> = cost_food.into_iter().map(esc).collect();
    format!(
        "{{\"points\":{},\"egg_limit\":{},\"wingspan\":{},\"nest\":{},\"habitats\":[{}],\"cost\":{{\"kind\":{},\"food\":[{}]}},\"power\":{},\"power_text\":{}}}",
        b.points, b.egg_limit, b.wingspan, esc(nest), habitats.join(","),
        esc(cost_kind), cost_food.join(","), esc(power), esc(&power_text(b)),
    )
}
