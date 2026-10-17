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
    grassland: false,
    wetland: false,
};

pub const HOODED_WARBLER: Bird = Bird {
    name: "Hooded Warbler",
    wingspan: 18,
    cost: COST,
    points: 7,
    niche: Niche::None,
    habitat: HABITAT,
    nest: Nest::Bowl,
    egg_limit: 3,
    power: Activation::None,
    shared_power: None,
};

