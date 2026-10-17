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
    fruits: 0,
    rodents: 0,
    wilds: 1,
});

const HABITAT: HabitatCollection = HabitatCollection {
    forest: false,
    grassland: false,
    wetland: true,
};

pub const TRUMPETER_SWAN: Bird = Bird {
    name: "Trumpeter Swan",
    wingspan: 203,
    cost: COST,
    points: 9,
    niche: Niche::None,
    habitat: HABITAT,
    nest: Nest::Ground,
    egg_limit: 2,
    power: Activation::None,
    shared_power: None,
};

