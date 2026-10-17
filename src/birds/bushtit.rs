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
    invertebrates: 1,
    seeds: 1,
    fish: 0,
    fruits: 0,
    rodents: 0,
    wilds: 0,
});

const HABITAT: HabitatCollection = HabitatCollection {
    forest: true,
    grassland: true,
    wetland: true,
};

const PLAY_BIRD: HabitatCollection = HabitatCollection::new();

const FOOD_CONSTRAINT: FoodCollection = FoodCollection::new(); 

const POWER: Activation = Activation::WhenActivated(Power {
    play_bird_in_this_habitat: false,
    play_bird: PLAY_BIRD,
    migration: false,
    hunt_bird: None,
    food_constraint: FOOD_CONSTRAINT,
    hunt_dice: false,
    feeder: false,
    gain_food: 0,
    discard_food: false,
    self_constraint: true,
    nest_constraint: Nest::None,
    lay_eggs_on_all: false,
    lay_eggs: 1,
    discard_egg: false,
    fewest_birds: None,
    face_up_constraint: false,
    deck_constraint: false,
    draw_bird: 0,
    tuck_or_cache: 1,
    discard_bird: true,
    repeat_power: None,
    draw_bonus: false,
});

pub const BUSHTIT: Bird = Bird {
    name: "Bushtit",
    wingspan: 15,
    cost: COST,
    points: 2,
    niche: Niche::Flocking,
    habitat: HABITAT,
    nest: Nest::Wild,
    egg_limit: 5,
    power: POWER,
    shared_power: None,
};

