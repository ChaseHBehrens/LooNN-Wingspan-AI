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
};

const COST: Cost = Cost::All(FoodCollection {
    invertebrates: 0,
    seeds: 1,
    fish: 0,
    fruits: 1,
    rodents: 0,
    wilds: 1,
});

const HABITAT: HabitatCollection = HabitatCollection {
    forest: false,
    grassland: false,
    wetland: true,
};

const PLAY_BIRD: HabitatCollection = HabitatCollection::new();

const FOOD_CONSTRAINT: FoodCollection = FoodCollection::new(); 

const POWER: Power = Power {
    play_bird_in_this_habitat: false,
    play_bird: PLAY_BIRD,
    migration: false,
    hunt_bird: None,
    food_constraint: FOOD_CONSTRAINT,
    hunt_dice: false,
    feeder: false,
    gain_food: 0,
    discard_food: false,
    self_constraint: false,
    nest_constraint: Nest::None,
    lay_eggs_on_all: false,
    lay_eggs: 0,
    discard_egg: false,
    fewest_birds: None,
    face_up_constraint: false,
    deck_constraint: true,
    draw_bird: 1,
    tuck_or_cache: 0,
    discard_bird: false,
    repeat_power: None,
    draw_bonus: false,
};

pub const PURPLE_GALLINULE: Bird = Bird {
    name: "Purple Gallinule",
    wingspan: 56,
    cost: COST,
    points: 7,
    niche: Niche::None,
    habitat: HABITAT,
    nest: Nest::Platform,
    egg_limit: 4,
    power: Activation::WhenActivated(POWER),
    shared_power: Some(POWER),
};

