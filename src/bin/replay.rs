//! Plays one 4-player game and prints a full replay as JSON: every player's
//! state after setup and after each turn, with the actions taken. Used by the
//! web viewer in `viewer/`.
//!
//! Usage: replay [loonn|randy x4]   (default: four LooNN players)

use std::collections::BTreeMap;
use std::env;
use std::fmt::Write;
use rand::rng;
use loonn::core::{
    Bird, BirdDealer, Evaluator, Feeder, Goal, GoalDealer, Player, ScoringEvent, Triggers, HABITATS,
};
use loonn::describe::{ bird_json, esc, food_list, power_text };
use loonn::simulation::{
    SimulatedBirdDeck, SimulatedBonusCardDeck, SimulatedFeeder, SimulatedGoalDeck,
};
use loonn::evaluators::{ LooNN, Randy };

const PLAYER_COUNT: usize = 4;

enum Evaluators { Randy(Randy), LooNN(LooNN) }

impl Evaluator for Evaluators {
    fn evaluate(&mut self, player: &Player) -> f32 {
        match self { Self::Randy(e) => e.evaluate(player), Self::LooNN(e) => e.evaluate(player) }
    }
    fn evaluate_all(&mut self, players: &[Player]) -> Vec<f32> {
        match self { Self::Randy(e) => e.evaluate_all(players), Self::LooNN(e) => e.evaluate_all(players) }
    }
    fn choose<T>(&mut self, options: impl Iterator<Item = (T, Player)>, _played: bool) -> Option<(T, Player)> {
        match self { Self::Randy(e) => e.choose(options, false), Self::LooNN(e) => e.choose(options, false) }
    }
}

fn player_json(p: &Player, goals: &[&'static Goal; 4], birds: &mut BTreeMap<&'static str, &'static Bird>) -> String {
    let mut s = String::from("{\"habitats\":{");
    for (i, h) in HABITATS.into_iter().enumerate() {
        if i > 0 { s.push(','); }
        let slots: Vec<String> = p.habitat(h).iter().map(|slot| {
            birds.insert(slot.bird.name, slot.bird);
            format!("{{\"bird\":{},\"eggs\":{}}}", esc(slot.bird.name), slot.eggs())
        }).collect();
        let _ = write!(s, "{}:[{}]", esc(&h.to_string()), slots.join(","));
    }
    s.push('}');
    let food: Vec<String> = food_list(&p.food()).into_iter().map(esc).collect();
    let hand: Vec<String> = p.hand().iter().map(|b| { birds.insert(b.name, b); esc(b.name) }).collect();
    let bonus: Vec<String> = p.bonus_cards().iter()
        .map(|b| format!("{{\"name\":{},\"score\":{}}}", esc(b.name), b.score(p)))
        .collect();
    let goal_scores: Vec<String> = goals.iter().map(|g| g.score(p).to_string()).collect();
    let eggs: usize = p.bird_slots().map(|s| s.eggs() as usize).sum();
    let _ = write!(
        s,
        ",\"food\":[{}],\"hand\":[{}],\"bonus\":[{}],\"goal_scores\":[{}],\"score\":{},\"eggs\":{},\"turns_left\":{}}}",
        food.join(","), hand.join(","), bonus.join(","), goal_scores.join(","),
        p.accumulated_score(), eggs, p.total_remaining_turns(),
    );
    s
}

fn tray_json(dealer: &mut SimulatedBirdDeck, birds: &mut BTreeMap<&'static str, &'static Bird>) -> String {
    let names: Vec<String> = dealer.face_up_birds().iter()
        .map(|b| { birds.insert(b.name, b); esc(b.name) })
        .collect();
    format!("[{}]", names.join(","))
}

fn feeder_json(feeder: &SimulatedFeeder) -> String {
    let d = feeder.contents();
    let mut v = Vec::new();
    for (n, name) in [
        (d.invertebrates, "invertebrate"), (d.seeds, "seed"), (d.fish, "fish"),
        (d.fruits, "fruit"), (d.rodents, "rodent"), (d.invertebrate_or_seeds, "invertebrate/seed"),
    ] {
        for _ in 0..n { v.push(esc(name)); }
    }
    format!("[{}]", v.join(","))
}

fn main() {
    // `replay --powers`: every bird's power text as JSON, so older saved games can show powers too
    if env::args().nth(1).as_deref() == Some("--powers") {
        let dealer = SimulatedBirdDeck::new(false);
        let rows: Vec<String> = dealer.deck()
            .map(|b| format!("{}:{}", esc(b.name), esc(&power_text(b))))
            .collect();
        println!("{{{}}}", rows.join(","));
        return;
    }
    let kinds: Vec<String> = (0..PLAYER_COUNT)
        .map(|i| env::args().nth(i + 1).unwrap_or_else(|| "loonn".into()).to_lowercase())
        .collect();

    let mut bird_dealer = SimulatedBirdDeck::new(false);
    let mut bonus_dealer = SimulatedBonusCardDeck::new(false);
    let mut goal_dealer = SimulatedGoalDeck::new(false);
    let mut feeder = SimulatedFeeder::new(false);
    let goals = goal_dealer.set_goals();
    let mut rng = rng();
    // Predicts every player's final score, whoever is choosing their moves
    let mut predictor = LooNN::with_bird_bonus(0.0).expect("Failed to construct LooNN");

    let mut evaluators: Vec<Evaluators> = kinds.iter().map(|k| match k.as_str() {
        "randy" => Evaluators::Randy(Randy::new()),
        _ => Evaluators::LooNN(LooNN::new().expect("Failed to construct LooNN")),
    }).collect();
    let mut players: Vec<Player> = evaluators.iter_mut()
        .map(|e| Player::new(e, goals, &mut bird_dealer, &mut bonus_dealer, false))
        .collect();
    let mut triggers: Vec<Triggers> = (0..PLAYER_COUNT).map(|_| Triggers::new()).collect();
    let mut birds: BTreeMap<&'static str, &'static Bird> = BTreeMap::new();
    let mut steps: Vec<String> = Vec::new();

    let snapshot = |players: &[Player], predictor: &mut LooNN, finished: &[bool],
                        tray: String, feeder: String, birds: &mut BTreeMap<&'static str, &'static Bird>,
                        head: String| -> String {
        let predictions = predictor.evaluate_all(players);
        let states: Vec<String> = players.iter().map(|p| player_json(p, &goals, birds)).collect();
        let predicted: Vec<String> = players.iter().zip(&predictions).zip(finished)
            .map(|((p, &pred), &done)| format!("{:.1}", if done { p.accumulated_score() as f32 } else { pred }))
            .collect();
        format!("{{{head},\"players\":[{}],\"predicted\":[{}],\"tray\":{tray},\"feeder\":{feeder}}}",
            states.join(","), predicted.join(","))
    };

    let mut finished = vec![false; PLAYER_COUNT];
    bird_dealer.refill_face_up_birds();
    let tray = tray_json(&mut bird_dealer, &mut birds);
    let head = format!(
        "\"kind\":\"setup\",\"player\":null,\"round\":0,\"actions\":[],\"tray_before\":{tray},\"feeder_before\":{},\"deck\":{},\"discard\":{}",
        feeder_json(&feeder), bird_dealer.deck_len(), bird_dealer.discard_len(),
    );
    let step = snapshot(&players, &mut predictor, &finished, tray, feeder_json(&feeder), &mut birds, head);
    steps.push(step);

    let mut i = 0;
    let mut round = 0;
    let mut game_end = 0;
    while game_end < PLAYER_COUNT {
        let mut round_end = 0;
        while round_end < PLAYER_COUNT {
            bird_dealer.refill_face_up_birds();
            for h in HABITATS {
                let mine = players[i].habitat(h).len();
                let fewest = players.iter().all(|p| p.habitat(h).len() >= mine);
                triggers[i].fewest_birds.set_habitat(h, fewest);
            }
            // What the player could see when choosing: the refilled tray and the feeder
            let board_before = format!(
                "\"tray_before\":{},\"feeder_before\":{}",
                tray_json(&mut bird_dealer, &mut birds), feeder_json(&feeder),
            );
            let score_before = players[i].accumulated_score();
            let turn = 27 - players[i].total_remaining_turns() as usize;
            let (result, actions) = players[i].play_turn_recorded(
                &mut evaluators[i], &mut feeder, &mut bird_dealer, &mut bonus_dealer,
                &triggers[i], &mut rng, false,
            );
            // Triggers are used up by the turn, as in train.rs and test.rs
            triggers[i] = Triggers::new();
            let action_lines: Vec<String> = actions.iter()
                .flat_map(|a| a.to_string().lines().map(esc).collect::<Vec<_>>())
                .collect();
            let kind = match result {
                Ok(()) => {
                    let new_triggers = Triggers::from(actions);
                    for (j, t) in triggers.iter_mut().enumerate() {
                        if j != i { *t += new_triggers.clone(); }
                    }
                    "turn"
                }
                Err(ScoringEvent::EndOfRound) => {
                    round_end += 1;
                    triggers[i] = Triggers::new();
                    "end_of_round"
                }
                Err(ScoringEvent::EndOfGame) => {
                    round_end += 1;
                    game_end += 1;
                    finished[i] = true;
                    triggers[i] = Triggers::new();
                    "end_of_game"
                }
            };
            let gained = players[i].accumulated_score() as i64 - score_before as i64;
            let tray = tray_json(&mut bird_dealer, &mut birds);
            let head = format!(
                "\"kind\":{},\"player\":{},\"round\":{},\"turn\":{},\"gained\":{},\"actions\":[{}],{board_before},\"deck\":{},\"discard\":{}",
                esc(kind), i, round, turn, gained, action_lines.join(","),
                bird_dealer.deck_len(), bird_dealer.discard_len(),
            );
            let step = snapshot(&players, &mut predictor, &finished, tray, feeder_json(&feeder), &mut birds, head);
            steps.push(step);
            i = (i + 1) % PLAYER_COUNT;
        }
        i = (i + 1) % PLAYER_COUNT;
        round += 1;
    }

    let names: Vec<String> = kinds.iter().map(|k| esc(if k == "randy" { "Randy" } else { "LooNN" })).collect();
    let goal_names: Vec<String> = goals.iter().map(|g| esc(g.name)).collect();
    let bird_defs: Vec<String> = birds.iter().map(|(n, b)| format!("{}:{}", esc(n), bird_json(b))).collect();
    println!(
        "{{\"players\":[{}],\"goals\":[{}],\"birds\":{{{}}},\"steps\":[{}]}}",
        names.join(","), goal_names.join(","), bird_defs.join(","), steps.join(","),
    );
}
