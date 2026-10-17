use inquire::{ InquireError, Select };
use crate::core::{ Feeder, Die, Dice };


/// User input for a feeder
pub struct UserInterfaceFeeder;

impl UserInterfaceFeeder {
    pub fn new() -> Self { Self }
    
    /// Gets `Dice` from the user via the terminal
    fn get_dice_input() -> Dice {
        let mut dice = Dice::new();
        for _ in 0..5 {
            match Self::get_die_input() {
                Ok(die) => dice.insert(die),
                Err(_) => return dice,
            }
        }
        dice
    }

    /// Gets a `Die` from the user via the terminal
    fn get_die_input() -> Result<Die, InquireError> {
        let options = vec![
            Die::Invertebrate,
            Die::Seed,
            Die::Fish,
            Die::Fruit,
            Die::Rodent,
            Die::InvertebrateOrSeed,
        ];
        Select::new(&format!("Enter die:"), options)
            .with_page_size(6)
            .prompt()
    }
}

impl Feeder for UserInterfaceFeeder {
    fn contents(&self) -> Dice {
        println!("Enter the dice in the feeder tray:");
        Self::get_dice_input()
    }

    fn take_die(&mut self, _die: crate::core::Die) { }

    fn hunt(&mut self) -> Dice {
        println!("Enter the result of rolling the dice outside of the feeder:");
        Self::get_dice_input()
    }

    fn randomize(&mut self) { }
}

