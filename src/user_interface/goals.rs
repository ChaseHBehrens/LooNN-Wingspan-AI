use inquire::{ InquireError, Select };
use crate::core::{ GoalDealer, Goal };
use crate::goals::GOALS;


/// User input for bonus card deck
pub struct UserInterfaceGoalDeck;

impl UserInterfaceGoalDeck {
    pub fn new() -> Self { Self }

    /// Gets a `&'static BonusCard` from the user via the terminal
    fn get_goal_input() -> Result<&'static Goal, InquireError> {
        Select::new(&format!("Enter goal name:"), GOALS.iter().collect())
            .with_page_size(5)
            .prompt()
    }
}

impl GoalDealer for UserInterfaceGoalDeck {
    fn set_goals(&mut self) -> [&'static Goal; 4] {
        println!("Enter the names of the end of round goals:");
        std::array::from_fn(|_| loop {
            if let Ok(goal) = Self::get_goal_input() {
                return goal;
            }
        })
    }
}
