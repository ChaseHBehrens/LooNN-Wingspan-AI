use paste::paste;
use crate::core::BonusCard;

macro_rules! card_modules {
    ($($name:ident),* $(,)?) => {
        $(
            pub mod $name;
            pub use $name::*;
        )*
        paste! {
            /// Represents the full deck of bonus cards
            pub static BONUSES: [BonusCard; <[()]>::len(&[$(card_modules!(@replace $name ())),*])] = [
                $($name::[< $name:upper >]),*
            ];
        }
    };
    (@replace $name:ident $sub:expr) => { $sub };
}

card_modules! {
    anatomist,
    backyard_birder,
    bird_counter,
    bird_feeder,
    breeding_manager,
    cartographer,
    ecologist,
    enclosure_builder,
    falconer,
    fishery_manager,
    food_web_expert,
    forester,
    historian,
    large_bird_specialist,
    nest_box_builder,
    omnivore_specialist,
    oologist,
    passerine_specialist,
    photographer,
    platform_builder,
    prairie_manager,
    rodentologist,
    visionary_leader,
    viticulturist,
    wetland_specialist,
    wildlife_gardener,
}

