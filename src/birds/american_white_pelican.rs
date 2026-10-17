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
    seeds: 0,
    fish: 2,
    fruits: 0,
    rodents: 0,
    wilds: 0,
});

const HABITAT: HabitatCollection = HabitatCollection {
    forest: false,
    grassland: false,
    wetland: true,
};

const PLAY_BIRD: HabitatCollection = HabitatCollection::new();

const FOOD_CONSTRAINT: FoodCollection = FoodCollection { 
    invertebrates: 0, 
    seeds: 0, 
    fish: 1, 
    fruits: 0, 
    rodents: 0, 
    wilds: 0,
};

const POWER: Activation = Activation::WhenActivated(Power {
    play_bird_in_this_habitat: false,
    play_bird: PLAY_BIRD,
    migration: false,
    hunt_bird: None,
    food_constraint: FOOD_CONSTRAINT,
    hunt_dice: false,
    feeder: false,
    gain_food: 0,
    discard_food: true,
    self_constraint: false,
    nest_constraint: Nest::None,
    lay_eggs_on_all: false,
    lay_eggs: 0,
    discard_egg: false,
    fewest_birds: None,
    face_up_constraint: false,
    deck_constraint: false,
    draw_bird: 0,
    tuck_or_cache: 2,
    discard_bird: false,
    repeat_power: None,
    draw_bonus: false,
});

pub const AMERICAN_WHITE_PELICAN: Bird = Bird {
    name: "American White Pelican",
    wingspan: 274,
    cost: COST,
    points: 5,
    niche: Niche::Flocking,
    habitat: HABITAT,
    nest: Nest::Ground,
    egg_limit: 1,
    power: POWER,
    shared_power: None,
};

