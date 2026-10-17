pub use crate::core::{ 
    Bird, 
    Cost,
    Nest, 
    Niche, 
    Activation, 
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
    wetland: false,
};

pub const BLUEWINGED_WARBLER: Bird = Bird {
    name: "Blue-Winged Warbler",
    wingspan: 20,
    cost: COST,
    points: 8,
    niche: Niche::None,
    habitat: HABITAT,
    nest: Nest::Bowl,
    egg_limit: 2,
    power: Activation::None,
    shared_power: None,
};

