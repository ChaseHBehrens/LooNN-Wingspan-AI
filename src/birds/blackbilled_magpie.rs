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
    Trigger,
};

const COST: Cost = Cost::All(FoodCollection {
    invertebrates: 0,
    seeds: 0,
    fish: 0,
    fruits: 0,
    rodents: 0,
    wilds: 2,
});

const HABITAT: HabitatCollection = HabitatCollection {
    forest: false,
    grassland: true,
    wetland: false,
};

const PLAY_BIRD: HabitatCollection = HabitatCollection::new();

const FOOD_CONSTRAINT: FoodCollection = FoodCollection::all(); 

const POWER: Activation = Activation::OnceBetweenTurns(Trigger::HuntSuccess, Power {
    play_bird_in_this_habitat: false,
    play_bird: PLAY_BIRD,
    migration: false,
    hunt_bird: None,
    food_constraint: FOOD_CONSTRAINT,
    hunt_dice: false,
    feeder: true,
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

pub const BLACKBILLED_MAGPIE: Bird = Bird {
    name: "Black-Billed Magpie",
    wingspan: 64,
    cost: COST,
    points: 3,
    niche: Niche::None,
    habitat: HABITAT,
    nest: Nest::Wild,
    egg_limit: 3,
    power: POWER,
    shared_power: None,
};

