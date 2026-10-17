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
    seeds: 2,
    fish: 0,
    fruits: 1,
    rodents: 0,
    wilds: 0,
});

const HABITAT: HabitatCollection = HabitatCollection {
    forest: true,
    grassland: true,
    wetland: false,
};

pub const WILD_TURKEY: Bird = Bird {
    name: "Wild Turkey",
    wingspan: 135,
    cost: COST,
    points: 8,
    niche: Niche::None,
    habitat: HABITAT,
    nest: Nest::Ground,
    egg_limit: 5,
    power: Activation::None,
    shared_power: None,
};

