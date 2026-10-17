use inquire::{ InquireError, Select };
use crate::core::{ BonusDealer, BonusCard };
use crate::bonuses::BONUSES;


/// User input for bonus card deck
pub struct UserInterfaceBonusCardDeck;

impl UserInterfaceBonusCardDeck {
    pub fn new() -> Self { Self }

    /// Gets a `&'static BonusCard` from the user via the terminal
    fn get_bonus_card_input() -> Result<&'static BonusCard, InquireError> {
        Select::new(&format!("Enter bonus card name:"), BONUSES.iter().collect())
            .with_page_size(5)
            .prompt()
    }
}

impl BonusDealer for UserInterfaceBonusCardDeck {
    fn draw(&mut self) -> &'static BonusCard {
        println!("Enter the name of the bonus card drawn from the deck:");
        loop {
            match Self::get_bonus_card_input() {
                Ok(bonus_card) => return bonus_card,
                Err(_) => continue,
            }
        }
    }

    fn discard(&mut self, _bonus: &'static BonusCard) { }
}
