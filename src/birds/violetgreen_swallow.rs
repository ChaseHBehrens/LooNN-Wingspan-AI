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
    invertebrates: 2,
    seeds: 0,
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
    self_constraint: false,
    nest_constraint: Nest::None,
    lay_eggs_on_all: false,
    lay_eggs: 0,
    discard_egg: false,
    fewest_birds: None,
    face_up_constraint: true,
    deck_constraint: true,
    draw_bird: 1,
    tuck_or_cache: 1,
    discard_bird: false,
    repeat_power: None,
    draw_bonus: false,
});

pub const VIOLETGREEN_SWALLOW: Bird = Bird {
    name: "Violet-Green Swallow",
    wingspan: 36,
    cost: COST,
    points: 3,
    niche: Niche::Flocking,
    habitat: HABITAT,
    nest: Nest::Cavity,
    egg_limit: 3,
    power: POWER,
    shared_power: None,
};

