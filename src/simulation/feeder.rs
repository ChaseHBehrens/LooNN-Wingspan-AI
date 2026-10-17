use crate::core::{ Feeder, Dice };
use rand::{ rng, rngs::ThreadRng };


/// Simulates the feeder
pub struct SimulatedFeeder {
    tray: Dice, 
    outside: u8,
    rng: ThreadRng,
    logged: bool,
}

impl SimulatedFeeder {
    pub fn new(logged: bool) -> Self {
        let mut rng = rng();
        Self {
            tray: Dice::random(&mut rng, 5),
            outside: 0, 
            rng,
            logged,
        }
    }
}


impl Feeder for SimulatedFeeder {
    fn contents(&self) -> Dice { self.tray.clone() }
    
    fn take_die(&mut self, die: crate::core::Die) {
        self.tray.remove(die);
        self.outside += 1;
    }

    fn hunt(&mut self) -> Dice { Dice::random(&mut self.rng, self.outside) }

    fn randomize(&mut self) {
        self.tray = Dice::random(&mut self.rng, 5);
        if self.logged { println!("Rolled {}", self.tray); }
        self.outside = 0;
    }
}

