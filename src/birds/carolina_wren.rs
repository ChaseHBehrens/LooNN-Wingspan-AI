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
    grassland: false,
    wetland: false,
};

const PLAY_BIRD: HabitatCollection = HabitatCollection::new();

const FOOD_CONSTRAINT: FoodCollection = FoodCollection::new(); 

const POWER: Activation = Activation::WhenPlayed(Power {
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
    face_up_constraint: true,
    deck_constraint: true,
    draw_bird: 2,
    tuck_or_cache: 0,
    discard_bird: false,
    repeat_power: None,
    draw_bonus: false,
});

pub const CAROLINA_WREN: Bird = Bird {
    name: "Carolina Wren",
    wingspan: 20,
    cost: COST,
    points: 1,
    niche: Niche::None,
    habitat: HABITAT,
    nest: Nest::Cavity,
    egg_limit: 5,
    power: POWER,
    shared_power: None,
};

