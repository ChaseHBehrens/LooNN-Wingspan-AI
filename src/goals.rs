use crate::core::{ Goal, Habitat, Nest, Player };


/// Represents all possible end of round goals
pub static GOALS: [Goal; 16] = [
    BIRDS_IN_FOREST,
    BIRDS_IN_GRASSLAND,
    BIRDS_IN_WETLAND,
    BIRDS_WITH_EGG_IN_BOWL,
    BIRDS_WITH_EGG_IN_CAVITY,
    BIRDS_WITH_EGG_ON_GROUND,
    BIRDS_WITH_EGG_ON_PLATFORM,
    EGGS_IN_FOREST,
    EGGS_IN_GRASSLAND,
    EGGS_IN_WETLAND,
    EGGS_IN_BOWL,
    EGGS_IN_CAVITY,
    EGGS_ON_GROUND,
    EGGS_ON_PLATFORM,
    TOTAL_BIRDS,
    EGG_SETS,
];

const BIRDS_IN_FOREST: Goal = Goal {
    name: "Birds In Forest",
    score: |player: &Player| player.habitat(Habitat::Forest).len().min(5),
};

const BIRDS_IN_GRASSLAND: Goal = Goal {
    name: "Birds In Grassland",
    score: |player: &Player| player.habitat(Habitat::Grassland).len().min(5),
};

const BIRDS_IN_WETLAND: Goal = Goal {
    name: "Birds In Wetland",
    score: |player: &Player| player.habitat(Habitat::Wetland).len().min(5),
};

const BIRDS_WITH_EGG_IN_BOWL: Goal = Goal {
    name: "Birds With Egg In Bowl",
    score: |player: &Player| player.nest(Nest::Bowl).iter().filter(|s| s.eggs() > 0).count().min(5),
};

const BIRDS_WITH_EGG_IN_CAVITY: Goal = Goal {
    name: "Birds With Egg In Cavity",
    score: |player: &Player| player.nest(Nest::Cavity).iter().filter(|s| s.eggs() > 0).count().min(5),
};

const BIRDS_WITH_EGG_ON_GROUND: Goal = Goal {
    name: "Birds With Egg On Ground",
    score: |player: &Player| player.nest(Nest::Ground).iter().filter(|s| s.eggs() > 0).count().min(5),
};

const BIRDS_WITH_EGG_ON_PLATFORM: Goal = Goal {
    name: "Birds With Egg On Platform",
    score: |player: &Player| player.nest(Nest::Platform).iter().filter(|s| s.eggs() > 0).count().min(5),
};

const EGGS_IN_FOREST: Goal = Goal {
    name: "Eggs In Forest",
    score: |player: &Player| player.habitat(Habitat::Forest).iter().map(|s| s.eggs() as usize).sum::<usize>().min(5),
};

const EGGS_IN_GRASSLAND: Goal = Goal {
    name: "Eggs In Grassland",
    score: |player: &Player| player.habitat(Habitat::Grassland).iter().map(|s| s.eggs() as usize).sum::<usize>().min(5),
};

const EGGS_IN_WETLAND: Goal = Goal {
    name: "Eggs In Wetland",
    score: |player: &Player| player.habitat(Habitat::Wetland).iter().map(|s| s.eggs() as usize).sum::<usize>().min(5),
};

const EGGS_IN_BOWL: Goal = Goal {
    name: "Eggs In Bowl",
    score: |player: &Player| player.nest(Nest::Bowl).iter().map(|s| s.eggs() as usize).sum::<usize>().min(5),
};

const EGGS_IN_CAVITY: Goal = Goal {
    name: "Eggs In Cavity",
    score: |player: &Player| player.nest(Nest::Cavity).iter().map(|s| s.eggs() as usize).sum::<usize>().min(5),
};

const EGGS_ON_GROUND: Goal = Goal {
    name: "Eggs On Ground",
    score: |player: &Player| player.nest(Nest::Ground).iter().map(|s| s.eggs() as usize).sum::<usize>().min(5),
};

const EGGS_ON_PLATFORM: Goal = Goal {
    name: "Eggs On Platform",
    score: |player: &Player| player.nest(Nest::Platform).iter().map(|s| s.eggs() as usize).sum::<usize>().min(5),
};

const TOTAL_BIRDS: Goal = Goal {
    name: "Total Birds",
    score: |player: &Player| player.habitat(Habitat::Forest).iter()
        .chain(player.habitat(Habitat::Grassland).iter())
        .chain(player.habitat(Habitat::Wetland).iter())
        .count().min(5),
};

const EGG_SETS: Goal = Goal {
    name: "Egg Sets",
    score: |player: &Player| player.habitat(Habitat::Forest).iter().map(|s| s.eggs() as usize).sum::<usize>()
        .min(player.habitat(Habitat::Grassland).iter().map(|s| s.eggs() as usize).sum())
        .min(player.habitat(Habitat::Wetland).iter().map(|s| s.eggs() as usize).sum())
        .min(5),
};

