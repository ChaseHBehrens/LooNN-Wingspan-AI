use rand::{ rng, RngExt, rngs::ThreadRng };
use crate::core::{ Goal, GoalDealer };
use crate::goals::GOALS;


/// Simulates the feeder
pub struct SimulatedGoalDeck {
    goals: Vec<&'static Goal>,
    rng: ThreadRng,
    logged: bool,
}

impl SimulatedGoalDeck {
    pub fn new(logged: bool) -> Self {
        let rng = rng();
        Self {
            goals: GOALS.iter().collect(),
            rng,
            logged,
        }
    }

    fn deal_goal(&mut self) -> &'static Goal {
        let i = self.rng.random_range(0..self.goals.len());
        self.goals.swap_remove(i)
    }
}

impl GoalDealer for SimulatedGoalDeck {
    fn set_goals(&mut self) -> [&'static Goal; 4] {
        let goals = std::array::from_fn(|_| self.deal_goal());
        if self.logged { for i in 0..4 {
            println!("Goal {} is {}", i + 1, goals[i]);
        }}
        goals
    }
}
