pub use crate::core::{ 
    Bird, 
    Cost,
    Nest, 
    Niche, 
    Activation, 
    Power, 
    Food, 
    FoodCollection, 
    HabitatCollection, 
    Habitat,
    Trigger,
};

const COST: Cost = Cost::OneOf(FoodCollection {
    invertebrates: 1,
    seeds: 0,
    fish: 0,
    fruits: 1,
    rodents: 0,
    wilds: 0,
});

const HABITAT: HabitatCollection = HabitatCollection {
    forest: true,
    grassland: true,
    wetland: true,
};

const PLAY_BIRD: HabitatCollection = HabitatCollection::new();

const FOOD_CONSTRAINT: FoodCollection = FoodCollection {
    invertebrates: 1,
    seeds: 0,
    fish: 0,
    fruits: 0,
    rodents: 0,
    wilds: 0,
}; 

const POWER: Activation = Activation::OnceBetweenTurns(Trigger::PlayBird(Habitat::Forest), Power {
    play_bird_in_this_habitat: false,
    play_bird: PLAY_BIRD,
    migration: false,
    hunt_bird: None,
    food_constraint: FOOD_CONSTRAINT,
    hunt_dice: false,
    feeder: false,
    gain_food: 1,
    discard_food: false,
    self_constraint: false,
    nest_constraint: Nest::None,
    lay_eggs_on_all: false,
    lay_eggs: 0,
    discard_egg: false,
    fewest_birds: None,
    face_up_constraint: false,
    deck_constraint: false,
    draw_bird: 0,
    tuck_or_cache: 0,
    discard_bird: false,
    repeat_power: None,
    draw_bonus: false,
});

pub const EASTERN_KINGBIRD: Bird = Bird {
    name: "Eastern Kingbird",
    wingspan: 38,
    cost: COST,
    points: 2,
    niche: Niche::None,
    habitat: HABITAT,
    nest: Nest::Bowl,
    egg_limit: 2,
    power: POWER,
    shared_power: None,
};

