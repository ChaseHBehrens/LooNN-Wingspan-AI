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
    fruits: 1,
    rodents: 0,
    wilds: 0,
});

const HABITAT: HabitatCollection = HabitatCollection {
    forest: false,
    grassland: true,
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
    nest_constraint: Nest::Cavity,
    lay_eggs_on_all: true,
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

pub const ASHTHROATED_FLYCATCHER: Bird = Bird {
    name: "Ash-Throated Flycatcher",
    wingspan: 30,
    cost: COST,
    points: 4,
    niche: Niche::None,
    habitat: HABITAT,
    nest: Nest::Cavity,
    egg_limit: 4,
    power: POWER,
    shared_power: None,
};

