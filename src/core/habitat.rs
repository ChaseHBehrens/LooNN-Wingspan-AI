use std::fmt;

/// Represents a habitat
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Habitat {
    Forest, 
    Grassland,
    Wetland,
}

impl fmt::Display for Habitat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Forest => "forest",
            Self::Grassland => "grassland",
            Self::Wetland => "wetland",
        };
        write!(f, "{name}")
    }
}


/// All of the different habitats
pub const HABITATS: [Habitat; 3] = [
    Habitat::Forest,
    Habitat::Grassland,
    Habitat::Wetland,
];


/// Represents a collection of multiple types of habitat
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HabitatCollection {
    pub forest: bool,
    pub grassland: bool,
    pub wetland: bool,
}

impl HabitatCollection {
    pub const fn new() -> Self {
        Self {
            forest: false,
            grassland: false,
            wetland: false,
        }
    }

    pub const fn all() -> Self {
        Self {
            forest: true,
            grassland: true,
            wetland: true,
        }
    }

    pub fn contains(&self, habitat: &Habitat) -> bool {
        match habitat {
            Habitat::Forest => self.forest,
            Habitat::Grassland => self.grassland,
            Habitat::Wetland => self.wetland,
        }
    }

    pub fn insert(&mut self, habitat: Habitat) {
        match habitat {
            Habitat::Forest => self.forest = true,
            Habitat::Grassland => self.grassland = true,
            Habitat::Wetland => self.wetland = true,
        }
    }

    /// Sets the bool value of a habitat
    pub fn set_habitat(&mut self, habitat: Habitat, val: bool) {
        match habitat {
            Habitat::Forest => self.forest = val,
            Habitat::Grassland => self.grassland = val,
            Habitat::Wetland => self.wetland = val,
        };
    }
}

impl From<Habitat> for HabitatCollection {
    fn from(habitat: Habitat) -> Self {
        match habitat {
            Habitat::Forest => Self {
                forest: true,
                grassland: false,
                wetland: false,
            },
            Habitat::Grassland => Self {
                forest: false,
                grassland: true,
                wetland: false,
            },
            Habitat::Wetland => Self {
                forest: false, 
                grassland: false,
                wetland: true,
            },
        }
    }
}

impl From<Option<Habitat>> for HabitatCollection {
    fn from(habitat: Option<Habitat>) -> Self {
        match habitat {
            Some(Habitat::Forest) => Self {
                forest: true,
                grassland: false,
                wetland: false,
            },
            Some(Habitat::Grassland) => Self {
                forest: false,
                grassland: true,
                wetland: false,
            },
            Some(Habitat::Wetland) => Self {
                forest: false, 
                grassland: false,
                wetland: true,
            },
            None => Self {
                forest: false,
                grassland: false,
                wetland: false,
            },
        }
    }
}

