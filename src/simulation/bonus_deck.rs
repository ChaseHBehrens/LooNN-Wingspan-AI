use rand::{ rng, RngExt, rngs::ThreadRng };
use crate::core::{ BonusDealer, BonusCard };
use crate::bonuses::BONUSES;


/// Simulates the bonus card deck
pub struct SimulatedBonusCardDeck {
    deck: Vec<&'static BonusCard>,
    rng: ThreadRng,
    logged: bool,
}

impl SimulatedBonusCardDeck {
    pub fn new(logged: bool) -> Self {
        Self { 
            deck: BONUSES.iter().collect(),
            rng: rng(),
            logged,
        }
    }
}

impl BonusDealer for SimulatedBonusCardDeck {
    fn draw(&mut self) -> &'static BonusCard {
        let i = self.rng.random_range(0..self.deck.len());
        let bonus = self.deck.swap_remove(i);
        if self.logged { println!("Draw {}", bonus) }
        bonus
    }

    fn discard(&mut self, bonus: &'static BonusCard) {
        self.deck.push(bonus);
    }
}

