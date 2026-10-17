use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::env;

/// Board features followed by per-card features summed over the hand
const INPUT_DIM: usize = 1713 + 108;

type State = (Vec<f32>, Vec<Vec<f32>>);

fn label_habitat_collection(prefix: &str) -> Vec<String> {
    vec![
        format!("{prefix}.forest"),
        format!("{prefix}.grassland"),
        format!("{prefix}.wetland"),
    ]
}

fn label_food_constraint(prefix: &str) -> Vec<String> {
    vec![
        format!("{prefix}.invertebrates"),
        format!("{prefix}.seeds"),
        format!("{prefix}.fish"),
        format!("{prefix}.fruits"),
        format!("{prefix}.rodents"),
    ]
}

fn label_food_collection(prefix: &str) -> Vec<String> {
    let mut v = label_food_constraint(prefix);
    v.push(format!("{prefix}.wilds"));
    v
}

fn label_food_supply(prefix: &str) -> Vec<String> {
    label_food_constraint(prefix)
}

fn label_nest(prefix: &str) -> Vec<String> {
    vec![
        format!("{prefix}.platform"),
        format!("{prefix}.bowl"),
        format!("{prefix}.cavity"),
        format!("{prefix}.ground"),
    ]
}

fn label_niche(prefix: &str) -> Vec<String> {
    vec![format!("{prefix}.predator"), format!("{prefix}.flocking")]
}

fn label_trigger(prefix: &str) -> Vec<String> {
    let mut v = Vec::new();
    v.push(format!("{prefix}.lay_egg"));
    v.extend(label_habitat_collection(&format!("{prefix}.play_bird_habitat")));
    v.extend(label_food_constraint(&format!("{prefix}.gain_food")));
    v.push(format!("{prefix}.hunt_success"));
    v
}

fn label_power(prefix: &str) -> Vec<String> {
    let mut v = Vec::new();
    v.push(format!("{prefix}.play_bird_in_this_habitat"));
    v.extend(label_habitat_collection(&format!("{prefix}.play_bird")));
    v.push(format!("{prefix}.migration"));
    v.push(format!("{prefix}.hunt_bird.present"));
    v.push(format!("{prefix}.hunt_bird.wingspan"));
    v.extend(label_food_constraint(&format!("{prefix}.food_constraint")));
    v.push(format!("{prefix}.hunt_dice"));
    v.push(format!("{prefix}.feeder"));
    v.push(format!("{prefix}.gain_food"));
    v.push(format!("{prefix}.discard_food"));
    v.push(format!("{prefix}.self_constraint"));
    v.extend(label_nest(&format!("{prefix}.nest_constraint")));
    v.push(format!("{prefix}.lay_eggs_on_all"));
    v.push(format!("{prefix}.lay_eggs"));
    v.push(format!("{prefix}.discard_egg"));
    v.extend(label_habitat_collection(&format!("{prefix}.fewest_birds")));
    v.push(format!("{prefix}.face_up_constraint"));
    v.push(format!("{prefix}.deck_constraint"));
    v.push(format!("{prefix}.draw_bird"));
    v.push(format!("{prefix}.tuck_or_cache"));
    v.push(format!("{prefix}.discard_bird"));
    v.push(format!("{prefix}.repeat_power.present"));
    v.extend(label_niche(&format!("{prefix}.repeat_power.niche")));
    v.push(format!("{prefix}.draw_bonus"));
    v
}

fn label_activation(prefix: &str) -> Vec<String> {
    let mut v = Vec::new();
    v.push(format!("{prefix}.type.when_activated"));
    v.push(format!("{prefix}.type.once_between_turns"));
    v.push(format!("{prefix}.type.when_played"));
    v.extend(label_trigger(&format!("{prefix}.trigger")));
    v.extend(label_power(&format!("{prefix}.power")));
    v
}

fn label_bird_slot(prefix: &str) -> Vec<String> {
    let mut v = vec![format!("{prefix}.occupied")];
    v.extend(label_bird(prefix));
    v.push(format!("{prefix}.eggs"));
    v.push(format!("{prefix}.egg_room"));
    v
}

fn label_bird(prefix: &str) -> Vec<String> {
    let mut v = Vec::new();
    v.extend(label_food_collection(&format!("{prefix}.cost.all")));
    v.extend(label_food_collection(&format!("{prefix}.cost.one_of")));
    v.push(format!("{prefix}.points"));
    v.extend(label_niche(&format!("{prefix}.niche")));
    v.extend(label_habitat_collection(&format!("{prefix}.habitat")));
    v.extend(label_nest(&format!("{prefix}.nest")));
    v.push(format!("{prefix}.egg_limit"));
    v.extend(label_activation(&format!("{prefix}.power")));
    v.extend(label_power(&format!("{prefix}.shared_power")));
    v
}


fn label_position() -> Vec<String> {
    let mut v = Vec::new();
    v.extend(label_habitat_collection("position.habitat"));
    for i in 0..5 {
        v.push(format!("position.slot_index[{i}]"));
    }
    v
}

fn vectorize_player_labels() -> Vec<String> {
    let mut v = Vec::new();
    for h in ["forest", "grassland", "wetland"] {
        for i in 0..5 {
            v.extend(label_bird_slot(&format!("{h}[{i}]")));
        }
    }
    v.extend(label_food_supply("food_supply"));
    v.extend(label_habitat_collection("fewest_birds"));
    v.push("hand.len".to_string());
    v.push("bonus_card_score".to_string());
    for i in 0..4 {
        v.push(format!("goal[{i}]"));
    }
    v.extend(label_position());
    for turn in 1..=26 {
        v.push(format!("total_remaining_turns[{turn}]"));
    }
    v.extend(label_bird("hand.cards"));

    debug_assert_eq!(
        v.len(),
        INPUT_DIM,
        "vectorize_player_labels() produced {} labels, expected {} — \
         it has drifted out of sync with vectorize_player()",
        v.len(),
        INPUT_DIM
    );
    v
}

fn saliency(py_module: &Py<PyModule>, state: State) -> Vec<f32> {
    Python::attach(|py| {
        py_module
            .bind(py)
            .getattr("saliency")
            .expect("loonn.py missing `saliency` function")
            .call1(state)
            .expect("python saliency() call failed")
            .extract::<Vec<f32>>()
            .expect("saliency() must return a list of floats")
    })
}

fn top_features(
    py_module: &Py<PyModule>,
    state: State,
    top_n: usize,
) -> Vec<(String, f32)> {
    let labels = vectorize_player_labels();
    let scores = saliency(py_module, state);
    debug_assert_eq!(
        labels.len(),
        scores.len(),
        "label count ({}) and saliency output count ({}) don't match",
        labels.len(),
        scores.len()
    );
    let mut paired: Vec<(String, f32)> = labels.into_iter().zip(scores).collect();
    paired.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap());
    paired.truncate(top_n);
    paired
}

fn top_features_averaged(
    py_module: &Py<PyModule>,
    states: &[State],
    top_n: usize,
) -> Vec<(String, f32)> {
    assert!(!states.is_empty(), "need at least one state to average over");
    let labels = vectorize_player_labels();
    let mut sums = vec![0.0f32; labels.len()];
    for state in states {
        let scores = saliency(py_module, state.clone());
        for (sum, score) in sums.iter_mut().zip(scores.iter()) {
            *sum += score;
        }
    }
    let n = states.len() as f32;
    let mut paired: Vec<(String, f32)> = labels
        .into_iter()
        .zip(sums.into_iter().map(|s| s / n))
        .collect();
    paired.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap());
    paired.truncate(top_n);
    paired
}

fn load_states_sampled(py_module: &Py<PyModule>, max_samples: usize) -> Vec<State> {
    Python::attach(|py| {
        py_module
            .bind(py)
            .getattr("sample_states")
            .expect("loonn.py missing `sample_states` function")
            .call1((max_samples,))
            .expect("python sample_states() call failed")
            .extract::<Vec<State>>()
            .expect("sample_states() must return a list of (board, hand) pairs")
    })
}

fn print_features(features: &[(String, f32)]) {
    for (i, (label, score)) in features.iter().enumerate() {
        println!("{:>3}. {:<55} {:>12.6}", i + 1, label, score);
    }
}

fn main() -> PyResult<()> {
    let args: Vec<String> = env::args().collect();
    let top_n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(INPUT_DIM);
    let max_samples: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2000);

    let py_module = Python::attach(|py| -> PyResult<Py<PyModule>> {
        let sys = PyModule::import(py, "sys")?;
        let nn_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/neural_network");
        sys.getattr("path")?.call_method1("append", (nn_dir,))?;
        let module = PyModule::import(py, "loonn")?;
        module.getattr("init")?.call0()?;
        Ok(module.unbind())
    })?;

    let states = load_states_sampled(&py_module, max_samples);
    println!("Using {} sampled state(s)\n", states.len());

    if states.is_empty() {
        eprintln!("No states found — nothing to do.");
        return Ok(());
    }

    if states.len() == 1 {
        let result = top_features(&py_module, states.into_iter().next().unwrap(), top_n);
        println!("Top {top_n} features (single state):");
        print_features(&result);
    } else {
        let result = top_features_averaged(&py_module, &states, top_n);
        println!("Top {top_n} features (averaged over {} states):", states.len());
        print_features(&result);
    }

    Ok(())
}
