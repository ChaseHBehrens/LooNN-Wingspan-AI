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
};

const COST: Cost = Cost::All(FoodCollection {
    invertebrates: 0,
    seeds: 0,
    fish: 1,
    fruits: 0,
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
    fewest_birds: Some(Habitat::Wetland),
    face_up_constraint: true,
    deck_constraint: true,
    draw_bird: 1,
    tuck_or_cache: 0,
    discard_bird: false,
    repeat_power: None,
    draw_bonus: false,
};

pub const COMMON_LOON: Bird = Bird {
    name: "Common Loon",
    wingspan: 117,
    cost: COST,
    points: 6,
    niche: Niche::None,
    habitat: HABITAT,
    nest: Nest::Ground,
    egg_limit: 1,
    power: Activation::WhenActivated(POWER),
    shared_power: Some(POWER),
};

