/// Represents a birds niche
#[derive(Clone, Copy, Debug)]
pub enum Niche {
    None,
    Predator,
    Flocking,
    Any,
}

impl PartialEq for Niche {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Niche::Any, _) | (_, Niche::Any) => true,
            (Niche::Predator, Niche::Predator) => true,
            (Niche::Flocking, Niche::Flocking) => true,
            (Niche::None, Niche::None) => true,
            _ => false,
        }
    }
}
