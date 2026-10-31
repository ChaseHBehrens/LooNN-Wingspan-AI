//! Turns a player into the vectors the network reads: a board vector, and one
//! vector per card in hand. Shared by the Python-backed and native evaluators.

use crate::core::{
    Activation, Bird, BirdSlot, Cost, FoodCollection, HabitatCollection, Nest, Niche, Player,
    Position, Power, Trigger, HABITATS,
};

pub type Board = Vec<f32>;
pub type Hand = Vec<Vec<f32>>;

fn vectorize_power(power: &Power) -> Vec<f32> {
    let mut v = Vec::<f32>::new();
    v.push(power.play_bird_in_this_habitat.into());
    v.extend(vectorize_habitat_collection(&power.play_bird));
    v.push(power.migration.into());
    if let Some(bird) = power.hunt_bird {
        v.push(1.0);
        v.push(bird as f32 / 300.0);
    } else {
        v.extend(vec![0.0, 0.0]);
    }
    v.extend(vectorize_food_constraint(&power.food_constraint));
    v.push(power.hunt_dice.into());
    v.push(power.feeder.into());
    v.push(power.gain_food as f32 / 2.0);
    v.push(power.discard_food.into());
    v.push(power.self_constraint.into());
    v.extend(vectorize_nest(&power.nest_constraint));
    v.push(power.lay_eggs_on_all.into());
    v.push(power.lay_eggs as f32 / 2.0);
    v.push(power.discard_egg.into());
    v.extend(vectorize_habitat_collection(&power.fewest_birds.into()));
    v.push(power.face_up_constraint.into());
    v.push(power.deck_constraint.into());
    v.push(power.draw_bird as f32 / 2.0);
    v.push(power.tuck_or_cache as f32 / 2.0);
    v.push(power.discard_bird.into());
    if let Some(niche) = power.repeat_power {
        v.push(1.0);
        v.extend(vectorize_niche(&niche))
    } else { v.extend(vec![0.0, 0.0, 0.0]); }
    v.push(power.draw_bonus.into());
    v
}

fn vectorize_habitat_collection(habitat_collection: &HabitatCollection) -> Vec<f32> {
    vec![
        habitat_collection.forest.into(),
        habitat_collection.grassland.into(),
        habitat_collection.wetland.into(),
    ]
}

fn vectorize_food_constraint(food: &FoodCollection) -> Vec<f32> {
    vec![
        food.invertebrates as f32,
        food.seeds as f32,
        food.fish as f32,
        food.fruits as f32,
        food.rodents as f32,
    ]
}

fn vectorize_food_collection(food: &FoodCollection) -> Vec<f32> {
    vec![
        food.invertebrates as f32 / 2.0,
        food.seeds as f32 / 2.0,
        food.fish as f32 / 2.0,
        food.fruits as f32 / 2.0,
        food.rodents as f32 / 2.0,
        food.wilds as f32 / 2.0,
    ]
}

/// Food that must be paid, followed by flags for foods that can be paid as 1 of
fn vectorize_cost(cost: &Cost) -> Vec<f32> {
    match cost {
        Cost::All(food) => [vectorize_food_collection(food), vec![0.0; 6]].concat(),
        Cost::OneOf(foods) => [
            vec![0.0; 6],
            vectorize_food_collection(foods).into_iter().map(|f| (f > 0.0).into()).collect(),
        ].concat(),
    }
}

fn vectorize_food_supply(food: &FoodCollection) -> Vec<f32> {
    vec![
        (food.invertebrates as f32 / 15.0).clamp(0.0, 1.0),
        (food.seeds as f32 / 15.0).clamp(0.0, 1.0),
        (food.fish as f32 / 15.0).clamp(0.0, 1.0),
        (food.fruits as f32 / 15.0).clamp(0.0, 1.0),
        (food.rodents as f32 / 15.0).clamp(0.0, 1.0),
    ]
    
}

fn vectorize_nest(nest: &Nest) -> Vec<f32> {
    match nest {
        Nest::Platform => vec![1.0, 0.0, 0.0, 0.0],
        Nest::Bowl     => vec![0.0, 1.0, 0.0, 0.0],
        Nest::Cavity   => vec![0.0, 0.0, 1.0, 0.0],
        Nest::Ground   => vec![0.0, 0.0, 0.0, 1.0],
        Nest::Wild     => vec![1.0, 1.0, 1.0, 1.0],
        Nest::None     => vec![0.0, 0.0, 0.0, 0.0],
    }
}

fn vectorize_niche(niche: &Niche) -> Vec<f32> {
    match niche {
        Niche::None     => vec![0.0, 0.0],
        Niche::Predator => vec![1.0, 0.0],
        Niche::Flocking => vec![0.0, 1.0],
        Niche::Any      => vec![1.0, 1.0],
    }
}

fn vectorize_trigger(trigger: &Trigger) -> Vec<f32> {
    let mut v = Vec::<f32>::new();
    match trigger {
        Trigger::LayEgg => {
            v.push(1.0);
            v.extend(vec![0.0; 3]);
            v.extend(vec![0.0; 5]);
            v.push(0.0);
        },
        Trigger::PlayBird(habitat) => {
            v.push(0.0);
            v.extend(vectorize_habitat_collection(&habitat.clone().into()));
            v.extend(vec![0.0; 5]);
            v.push(0.0);
        },
        Trigger::GainFood(food) => {
            v.push(0.0);
            v.extend(vec![0.0; 3]);
            v.extend(vectorize_food_constraint(&food.clone().into()));
            v.push(0.0);
        },
        Trigger::HuntSuccess => {
            v.push(0.0);
            v.extend(vec![0.0; 3]);
            v.extend(vec![0.0; 5]);
            v.push(1.0);
        },
    }
    v
}

fn vectorize_activation(activation: &Activation) -> Vec<f32> {
    let mut v = Vec::<f32>::new();
    match activation {
        Activation::None => {
            v.extend(vec![0.0, 0.0, 0.0]);
            v.extend(vec![0.0; 10]);
            v.extend(vec![0.0; 36]);
        },
        Activation::WhenActivated(power) => {
            v.extend(vec![1.0, 0.0, 0.0]);
            v.extend(vec![0.0; 10]);
            v.extend(vectorize_power(power));
        },
        Activation::OnceBetweenTurns(trigger, power) => {
            v.extend(vec![0.0, 1.0, 0.0]);
            v.extend(vectorize_trigger(trigger));
            v.extend(vectorize_power(power));
        },
        Activation::WhenPlayed(power) => {
            v.extend(vec![0.0, 0.0, 1.0]);
            v.extend(vec![0.0; 10]);
            v.extend(vectorize_power(power));
        },
    }
    v
}

/// Length of a vectorized bird
pub const BIRD_DIM: usize = 108;

fn vectorize_bird(bird: &Bird) -> Vec<f32> {
    let mut v = Vec::<f32>::new();
    v.extend(vectorize_cost(&bird.cost));
    v.push(bird.points as f32 / 9.0);
    v.extend(vectorize_niche(&bird.niche));
    v.extend(vectorize_habitat_collection(&bird.habitat));
    v.extend(vectorize_nest(&bird.nest));
    v.push(bird.egg_limit as f32 / 6.0);
    v.extend(vectorize_activation(&bird.power));
    if let Some(power) = bird.shared_power {
        v.extend(vectorize_power(&power))
    } else {
        v.extend(vec![0.0; 36])
    }
    debug_assert_eq!(v.len(), BIRD_DIM);
    v
}

/// One vector per card; the network pools these with attention
fn vectorize_hand(hand: &Vec<&'static Bird>) -> Hand {
    hand.iter().map(|bird| vectorize_bird(bird)).collect()
}

/// Length of a vectorized bird slot, including the occupied flag
const BIRD_SLOT_DIM: usize = 1 + BIRD_DIM + 2;

fn vectorize_bird_slot(slot: &BirdSlot) -> Vec<f32> {
    let mut v = Vec::<f32>::new();
    v.push(1.0);
    v.extend(vectorize_bird(slot.bird));
    v.push(slot.eggs() as f32 / 6.0);
    v.push((slot.bird.egg_limit - slot.eggs()) as f32 / 6.0);
    v
}

fn vectorize_position(player: &Player) -> Vec<f32> {
    let mut v = Vec::<f32>::new();
    match player.activating() {
        Position::Triggers => v.extend(vec![0.0; 8]),
        Position::Played(h) => {
            v.extend(vectorize_habitat_collection(&h.clone().into()));
            match player.habitat(*h).len() {
                1 => v.extend(vec![1.0, 0.0, 0.0, 0.0, 0.0]),
                2 => v.extend(vec![0.0, 1.0, 0.0, 0.0, 0.0]),
                3 => v.extend(vec![0.0, 0.0, 1.0, 0.0, 0.0]),
                4 => v.extend(vec![0.0, 0.0, 0.0, 1.0, 0.0]),
                5 => v.extend(vec![0.0, 0.0, 0.0, 0.0, 1.0]),
                _ => unreachable!("invalid index")
            }
        },
        Position::Chain((h, i)) => {
            v.extend(vectorize_habitat_collection(&h.clone().into()));
            match i {
                0 => v.extend(vec![1.0, 0.0, 0.0, 0.0, 0.0]),
                1 => v.extend(vec![0.0, 1.0, 0.0, 0.0, 0.0]),
                2 => v.extend(vec![0.0, 0.0, 1.0, 0.0, 0.0]),
                3 => v.extend(vec![0.0, 0.0, 0.0, 1.0, 0.0]),
                4 => v.extend(vec![0.0, 0.0, 0.0, 0.0, 1.0]),
                _ => unreachable!("invalid index")
            }
        },
    }
    v
}

/// Returns the board vector and the per-card hand vectors
pub fn vectorize_player(player: &Player) -> (Board, Hand) { 
    let mut v = Vec::<f32>::new();
    for h in HABITATS { for i in 0..5 {
        if let Some(s) = player.habitat(h).get(i) {
            v.extend(vectorize_bird_slot(s));
        } else {
            v.extend(vec![0.0; BIRD_SLOT_DIM])
        }
    }}
    v.extend(vectorize_food_supply(&player.food()));
    v.extend(vectorize_habitat_collection(&player.fewest_birds()));
    v.push((player.hand().len() as f32 / 30.0).clamp(0.0, 1.0));
    v.push(player.bonus_cards()
        .iter()
        .map(|&b| b.score(player))
        .sum::<usize>() 
        as f32 / 80.0
    );
    let total_remaining_turns = player.total_remaining_turns();    
    v.extend(player.goals()
        .iter()
        .zip([18, 11, 5, 0])
        .map(|(&g, t)| if total_remaining_turns > t { 
            g.score(player) as f32 / 5.0 
        } else { 0.0 })
    );
    v.extend(vectorize_position(player));
    for turn in 1..=26 { v.push(if turn == total_remaining_turns { 1.0 } else { 0.0 }); }
    (v, vectorize_hand(player.hand()))
}
