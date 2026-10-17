
/// Represents a type of nest
#[derive(Debug, Clone, Copy)]
pub enum Nest {
    Platform,
    Bowl,
    Cavity,
    Ground,
    Wild,
    None,
}

impl PartialEq for Nest {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Nest::Wild, _) | (_, Nest::Wild) => true,
            (Nest::Platform, Nest::Platform) => true,
            (Nest::Bowl, Nest::Bowl) => true,
            (Nest::Cavity, Nest::Cavity) => true,
            (Nest::Ground, Nest::Ground) => true,
            (Nest::None, Nest::None) => true,
            _ => false,
        }
    }
}

