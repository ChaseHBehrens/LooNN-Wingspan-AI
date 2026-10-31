use std::{ ptr, array };
use rand::{ seq::IteratorRandom, RngExt };

use super::{
    Habitat, HabitatCollection, HABITATS,
    Food, FoodCollection, FOODS, Cost, Feeder, Dice, 
    BonusCard, BonusDealer,
    Goal,
    Nest,
    Bird, BirdDealer, Activation,
    Power, Trigger, Triggers,
    Action, PlayBird,
    Evaluator,
};

/// Random rollouts averaged when evaluating the gain food and draw birds actions
const LOOKAHEAD_SAMPLES: usize = 20;

/// Represents a scoring event
pub enum ScoringEvent {
    EndOfRound,
    EndOfGame,
}

/// Represents a slot of a player mat
#[derive(Clone)]
pub struct BirdSlot {
    pub bird: &'static Bird,
    eggs: u8,
} 

impl BirdSlot {
    fn new(bird: &'static Bird) -> Self {
        Self { 
            bird, 
            eggs: 0,
        }
    }
    
    /// Returns true if an egg can be laid on this slot
    fn can_fit_egg(&self) -> bool {
        (self.bird.egg_limit - self.eggs) > 0
    }

    /// Returns the number of eggs on the slot
    pub fn eggs(&self) -> u8 { self.eggs }
}

/// Represents the current activation
#[derive(Clone)]
pub enum Position {
    Triggers,
    Played(Habitat),
    Chain((Habitat, usize)),
}


/// Represents all information for a player
#[derive(Clone)]
pub struct Player {
    forest: Vec<BirdSlot>,
    grassland: Vec<BirdSlot>,
    wetland: Vec<BirdSlot>,
    bonus_cards: Vec<&'static BonusCard>,
    hand: Vec<&'static Bird>,
    accumulated_score: usize,
    food: FoodCollection,
    total_remaining_turns: u8,
    goals: [&'static Goal; 4],
    activating: Position,
    fewest_birds: HabitatCollection,
    scored: bool,
}

impl Player {
    pub fn new(
        evaluator: &mut impl Evaluator,
        goals: [&'static Goal; 4],
        bird_dealer: &mut impl BirdDealer,
        bonus_dealer: &mut impl BonusDealer,
        logged: bool,
    ) -> Self {
        let player = Self {
            forest: Vec::with_capacity(5),
            grassland: Vec::with_capacity(5),
            wetland: Vec::with_capacity(5),
            bonus_cards: Vec::new(),
            hand: Vec::new(),
            accumulated_score: 0,
            food: FoodCollection::new(),
            total_remaining_turns: 26,
            goals,
            activating: Position::Triggers,
            fewest_birds: HabitatCollection::all(),
            scored: false,
        };
        let dealt_hand: [&'static Bird; 5] = array::from_fn(|_| bird_dealer.draw_from_deck());
        let dealt_bonuses: [&'static BonusCard; 2] = array::from_fn(|_| bonus_dealer.draw());
        let mut best_player = player.clone();
        let mut best_evaluation = f32::NEG_INFINITY;
        for bonus in dealt_bonuses {
            for i in 0b0000011111_u16..=0b1111100000_u16 {
                if i.count_ones() != 5 { continue; } 
                let mut p = player.clone();
                p.bonus_cards.push(bonus);
                if (i & (1 << 0)) != 0 { p.food += Food::Invertebrate }
                if (i & (1 << 1)) != 0 { p.food += Food::Seed }
                if (i & (1 << 2)) != 0 { p.food += Food::Fish }
                if (i & (1 << 3)) != 0 { p.food += Food::Fruit }
                if (i & (1 << 4)) != 0 { p.food += Food::Rodent }
                if (i & (1 << 5)) != 0 { p.hand.push(dealt_hand[0]) }
                if (i & (1 << 6)) != 0 { p.hand.push(dealt_hand[1]) }
                if (i & (1 << 7)) != 0 { p.hand.push(dealt_hand[2]) }
                if (i & (1 << 8)) != 0 { p.hand.push(dealt_hand[3]) }
                if (i & (1 << 9)) != 0 { p.hand.push(dealt_hand[4]) }
                let evaluation = evaluator.evaluate(&p);
                if evaluation > best_evaluation {
                    best_player = p;
                    best_evaluation = evaluation;
                }
            }
        }
        for bird in dealt_hand.into_iter().filter(|b| 
            !best_player.hand.iter().any(|hand_bird| ptr::eq(*b, *hand_bird))
        ) { bird_dealer.discard(bird); }
        let discarded_bonus = if ptr::eq(best_player.bonus_cards[0], dealt_bonuses[0]) {
            dealt_bonuses[1]
        } else {
            dealt_bonuses[0]
        };
        bonus_dealer.discard(discarded_bonus);
        if logged {
            for bird in dealt_hand.into_iter().filter(|b| {
                !best_player.hand.iter().any(|hb| ptr::eq(*hb, *b))
            }) {
                println!("Discard {}", bird);
            }
            for food in FOODS.into_iter().filter(|&f| !best_player.food.contains(f.into())) {
                println!("Discard {}", food);
            }
            println!("Discard {}", discarded_bonus)
        }
        best_player
    }

    /// Returns an iterator of all played birds
    pub fn bird_slots(&self) -> impl Iterator<Item=&BirdSlot> {
        self.forest.iter()
            .chain(self.grassland.iter())
            .chain(self.wetland.iter())
    }
    
    /// Returns the slot of a given Bird
    fn slot_mut(&mut self, bird: &'static Bird) -> &mut BirdSlot {
        self.forest.iter_mut()
            .chain(self.grassland.iter_mut())
            .chain(self.wetland.iter_mut())
            .find(|slot| std::ptr::eq(slot.bird, bird))
            .expect(&format!("{} is not in play", bird.name))
    }

    /// Returns all filled slots in a habitat
    fn habitat_mut(&mut self, habitat: Habitat) -> &mut Vec<BirdSlot> {
        match habitat {
            Habitat::Forest => &mut self.forest,
            Habitat::Grassland => &mut self.grassland,
            Habitat::Wetland => &mut self.wetland,
        }
    }
    
    /// Returns all filled slots in a habitat
    pub fn habitat(&self, habitat: Habitat) -> &Vec<BirdSlot> {
        match habitat {
            Habitat::Forest => &self.forest,
            Habitat::Grassland => &self.grassland,
            Habitat::Wetland => &self.wetland,
        }
    }
    
    /// Returns the activating position
    pub fn activating(&self) -> &Position { &self.activating }
    
    /// Returns the habitat the activating bird is in
    pub fn active_habitat(&self) -> Option<Habitat> {
        match self.activating {
            Position::Triggers => None,
            Position::Played(habitat) => Some(habitat),
            Position::Chain((habitat, _)) => Some(habitat),
        }
    }
    
    /// Returns the bird that is activating
    pub fn active_bird(&self) -> Option<&'static Bird> {
        let (habitat, i) = match self.activating {
            Position::Triggers => return None,
            Position::Played(h) => (h, self.habitat(h).len() - 1),
            Position::Chain(p) => p,
        };
        Some(self.habitat(habitat)[i].bird)
    }
    
    /// Returns all slots with birds of type `nest`
    pub fn nest(&self, nest: Nest) -> Vec<&BirdSlot> {
        self.forest.iter()
            .chain(self.grassland.iter())
            .chain(self.wetland.iter())
            .filter(|s| s.bird.nest == nest)
            .collect()
    }

    /// Returns the birds a power may lay eggs on. A nest constraint of `Nest::None` means this bird only
    fn egg_targets(&self, power: &Power) -> Vec<&'static Bird> {
        let active_bird = self.active_bird();
        if matches!(power.nest_constraint, Nest::None) {
            let active_bird = active_bird.expect("No active bird");
            return self.bird_slots()
                .filter(|s| ptr::eq(s.bird, active_bird) && s.can_fit_egg())
                .map(|s| s.bird)
                .collect();
        }
        self.nest(power.nest_constraint)
            .iter()
            .filter(|s| s.can_fit_egg())
            .map(|s| s.bird)
            .filter(|&b| power.self_constraint || !active_bird.is_some_and(|a| ptr::eq(b, a)))
            .collect()
    }
     
    /// Returns the birds that can be placed in a habitat
    fn playable_birds_in_habitat(&self, habitat: Habitat) -> Vec<&'static Bird> {
        if self.habitat(habitat).len() == 5 { return Vec::new() }
        let eggs_needed = [0, 1, 1, 2, 2][match habitat {
            Habitat::Forest => self.forest.len(),
            Habitat::Grassland => self.grassland.len(),
            Habitat::Wetland => self.wetland.len(),
        }];
        if eggs_needed > self.total_eggs() { return Vec::new() }
        self.hand
            .iter()
            .filter(|b| 
                b.habitat.contains(&habitat) && 
                // HACK: ignores spending 2 foods as a wild
                b.cost.payment_options().into_iter().any(|o| self.food.contains(o)))
            .copied()
            .collect()
    }

    /// Returns the total number of eggs
    fn total_eggs(&self) -> u8 {
        self.forest.iter()
            .chain(&self.grassland)
            .chain(&self.wetland)
            .map(|s| s.eggs).sum()
    }

    /// Returns the food supply
    pub fn food(&self) -> FoodCollection { self.food }
    
    /// Returns the activating end of round goal
    fn scorable_goal(&self) -> Option<&'static Goal> {
        if self.scored { return None }
        Some(self.goals[match self.total_remaining_turns {
            18 => 0,
            11 => 1,
            5  => 2,
            0  => 3,
            _  => return None,
        }])
    }
    
    /// Returns the number of turns remaining in the game
    pub fn total_remaining_turns(&self) -> u8 { self.total_remaining_turns }

    /// Returns the current score
    pub fn accumulated_score(&self) -> usize { self.accumulated_score }
    
    /// Returns the bonus cards
    pub fn bonus_cards(&self) -> &Vec<&'static BonusCard> { &self.bonus_cards }

    /// Returns the end of round goals
    pub fn goals(&self) -> &[&'static Goal; 4] { &self.goals }

    /// Returns the cards in hand
    pub fn hand(&self) -> &Vec<&'static Bird> { &self.hand }
    
    /// Returns the habitats where this player has the fewest birds
    pub fn fewest_birds(&self) -> HabitatCollection { self.fewest_birds }

    /// Removes a bird from the hand
    fn remove_from_hand(&mut self, bird: &'static Bird) {
        let index = self.hand
            .iter()
            .position(|&b| std::ptr::eq(b, bird))
            .expect(&format!("{} is not in hand", bird.name));
        self.hand.swap_remove(index);
    }
    
    /// Removes an egg from the bird
    fn remove_egg(&mut self, bird: &'static Bird) {
        let slot = self.slot_mut(bird);
        debug_assert!(slot.eggs > 0, "{} does not have any eggs", bird.name);
        slot.eggs -= 1; 
    }

    /// Adds an egg to the bird
    fn lay_egg(&mut self, bird: &'static Bird) {
        let slot = self.slot_mut(bird);
        slot.eggs += 1;
        debug_assert!(slot.eggs <= bird.egg_limit, "{} exceeded egg limit", bird.name);
    } 

    /// Plays the bird into the given habitat
    fn play_bird(&mut self, bird: &'static Bird, habitat: Habitat) {
        debug_assert!(self.habitat(habitat).len() < 5, "Habitat is full");
        self.remove_from_hand(bird);
        self.habitat_mut(habitat).push(BirdSlot::new(bird));
        self.accumulated_score += bird.points as usize;
    }
    
    /// Updates the player based on the given turn
    pub fn play_turn(
        &mut self, 
        evaluator: &mut impl Evaluator,
        feeder: &mut impl Feeder,
        bird_dealer: &mut impl BirdDealer,
        bonus_dealer: &mut impl BonusDealer,
        triggers: &Triggers,
        rng: &mut impl RngExt,
        logged: bool,
    ) -> Result<Triggers, ScoringEvent> {
        let (result, actions) = self.play_turn_recorded(
            evaluator, feeder, bird_dealer, bonus_dealer, triggers, rng, logged
        );
        result.map(|()| Triggers::from(actions))
    }

    /// Like `play_turn`, but returns every action taken (including between-turn
    /// triggers, even on a scoring event) instead of the triggers they cause
    pub fn play_turn_recorded(
        &mut self, 
        evaluator: &mut impl Evaluator,
        feeder: &mut impl Feeder,
        bird_dealer: &mut impl BirdDealer,
        bonus_dealer: &mut impl BonusDealer,
        triggers: &Triggers,
        rng: &mut impl RngExt,
        logged: bool,
    ) -> (Result<(), ScoringEvent>, Vec<Action>) {
        if logged { println!("\x1b[33m-\x1b[0m Running Triggers"); }
        let mut actions = Vec::<Action>::new();
        let trigger_actions = self.run_triggers(
            evaluator, feeder, bird_dealer, bonus_dealer, triggers
        );
        if logged { for action in trigger_actions.iter() { println!("{}", action); } }
        actions.extend(trigger_actions);
        if let Some(goal) = self.scorable_goal() {
            if logged { println!("\x1b[33m-\x1b[0m Scoring End of Round"); }
            self.scored = true;
            self.accumulated_score += goal.score(&self);
            if self.total_remaining_turns != 0 {
                return (Err(ScoringEvent::EndOfRound), actions);
            }
        } 
        self.scored = false;
        if self.total_remaining_turns == 0 { 
            if logged { println!("\x1b[33m-\x1b[0m Scoring End of Game"); }
            self.accumulated_score += self.total_eggs() as usize;
            for bonus in self.bonus_cards.iter() {
                self.accumulated_score += bonus.score(&self);
            }
            return (Err(ScoringEvent::EndOfGame), actions);
        }
        if logged { println!("\x1b[33m-\x1b[0m Running Turn"); }
        let initial_actions = self.run_initial(
            evaluator, feeder, bird_dealer, rng
        );
        if logged { for action in initial_actions.iter() { println!("{}", action); } }
        actions.extend(initial_actions); 
        actions.extend(self.run_when_played(
            evaluator, feeder, bird_dealer, bonus_dealer, logged
        )); 
        actions.extend(self.run_when_activated(
            evaluator, feeder, bird_dealer, bonus_dealer, logged
        )); 
        self.total_remaining_turns -= 1;
        (Ok(()), actions)
    }

    fn run_triggers(
        &mut self,
        evaluator: &mut impl Evaluator,
        feeder: &mut impl Feeder,
        bird_dealer: &mut impl BirdDealer,
        bonus_dealer: &mut impl BonusDealer,
        triggers: &Triggers,
    ) -> Vec<Action> {
        self.fewest_birds = triggers.fewest_birds;
        let mut run = |t: Trigger| -> Vec<Action> {
            let powers: Vec<(Position, &Power)> = HABITATS
                .into_iter()
                .flat_map(|h| self.habitat(h).iter().enumerate().map(move |(i, s)| (h, i, s)))
                .filter_map(|(h, i, s)| {
                    let Activation::OnceBetweenTurns(ref trigger, ref power) = s.bird.power
                        else { return None; };
                    if *trigger != t { return None; }
                    Some((Position::Chain((h, i)), power))
                })
                .collect();
            powers.into_iter()
                .filter_map(|(position, power)| {
                    self.activating = position;
                    self.run_power(evaluator, feeder, bird_dealer, bonus_dealer, power)
                })
                .flatten()
                .collect()
        };
        let mut actions = Vec::<Action>::new();
        if triggers.lay_egg { actions.extend(run(Trigger::LayEgg)); }
        for h in HABITATS {
            if triggers.play_bird.contains(&h) { 
                actions.extend(run(Trigger::PlayBird(h))); 
            }
        };
        for f in FOODS {
            if triggers.gain_food.contains(f.into()) { 
                actions.extend(run(Trigger::GainFood(f))); 
            }
        };
        if triggers.hunt_success { actions.extend(run(Trigger::HuntSuccess)); }
        for power in triggers.opposing_bird.iter() {
            if let Some(a) = self.run_power(
                evaluator, feeder, bird_dealer, bonus_dealer, power
            ) { actions.extend(a); }
        }
        actions
    }
    
    fn run_initial(
        &mut self,
        evaluator: &mut impl Evaluator,
        feeder: &mut impl Feeder,
        bird_dealer: &mut impl BirdDealer,
        rng: &mut impl RngExt,
    ) -> Vec<Action> {
        let mut actions = Vec::<Action>::new();
        let mut options = Vec::<(InitialOption, f32)>::new();
        
        if let Some(option) = self.evaluate_initial_play_bird(
            evaluator,
        ) { options.push(option); }
        if let Some(option) = self.evaluate_initial_gain_food(
            evaluator, feeder, rng, false,
        ) { options.push(option); }
        if (self.forest.len() & 1) == 1 {
            if let Some(option) = self.evaluate_initial_gain_food(
                evaluator, feeder, rng, true,
            ) { options.push(option); }
        }
        if let Some(option) = self.evaluate_initial_lay_eggs(
            evaluator, false,
        ) { options.push(option); }
        if (self.grassland.len() & 1) == 1 {
            if let Some(option) = self.evaluate_initial_lay_eggs(
                evaluator, true,
            ) { options.push(option); }
        }
        if let Some(option) = self.evaluate_initial_draw_birds(
            evaluator, bird_dealer, rng, false,
        ) { options.push(option); }
        if (self.wetland.len() & 1) == 1 {
            if let Some(option) = self.evaluate_initial_draw_birds(
                evaluator, bird_dealer, rng, true,
            ) { options.push(option); }
        }

        // Exploration: sometimes take a random kind of action (each kind equally
        // likely, however many variants it has), still taking it the best way
        if rng.random_range(0.0..1.0) < evaluator.random_action_rate() {
            let mut kinds: Vec<_> = options.iter().map(|o| std::mem::discriminant(&o.0)).collect();
            kinds.dedup();
            if let Some(kind) = kinds.into_iter().choose(rng) {
                options.retain(|o| std::mem::discriminant(&o.0) == kind);
            }
        }
        let best_option = options
            .into_iter()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .expect("No Possible Actions");
        let best_option = best_option.0;
        let activation = best_option.activation();
        // Gaining food records each food as it is taken from the feeder; the other
        // actions were decided while evaluating options, so their result is recorded here
        let record = !matches!(best_option, InitialOption::GainFood { .. });
        match best_option {
            InitialOption::PlayBird { player, play_bird } => {
                *self = player;
                actions.push(Action::PlayBird(play_bird));
            }
            InitialOption::GainFood { player, discard_bird, food_gained } => {
                *self = player;
                if let Some(bird) = discard_bird {
                    actions.push(Action::DiscardBird(bird))
                }
                for _ in 0..food_gained {
                    actions.push(Action::InitialGainFood(
                        self.choose_from_feeder(evaluator, FoodCollection::all(), feeder)
                            .unwrap_or_else(|| unreachable!())
                    ));
                }
            },
            InitialOption::LayEggs { player, discard_food, birds } => {
                *self = player;
                if let Some(food) = discard_food {
                    actions.push(Action::DiscardFood(food))
                }
                for bird in birds {
                    actions.push(Action::InitialLayEgg(bird))
                }
            },
            InitialOption::DrawBirds { player, discard_egg, birds_drawn } => {
                *self = player;
                if let Some(bird) = discard_egg {
                    actions.push(Action::DiscardEgg(bird))
                }
                for _ in 0..birds_drawn {
                    actions.push(Action::DrawBird(
                        self.choose_bird_to_draw(evaluator, bird_dealer)
                    ));
                }
            },
        }
        self.activating = activation;
        if record { self.record(evaluator); }
        actions
    }

    /// Records the current state as one reached in play, for training
    fn record(&self, evaluator: &mut impl Evaluator) {
        let _ = evaluator.choose(std::iter::once(((), self.clone())), true);
    }

    fn evaluate_initial_play_bird(
        &self, 
        evaluator: &mut impl Evaluator,
    ) -> Option<(InitialOption, f32)> {
        let mut player = self.clone();
        if let Some(play_bird) = player.choose_bird_to_play(
            evaluator, 
            HabitatCollection::all(), 
            false
        ) {
            let score = evaluator.evaluate(&player);
            return Some((InitialOption::PlayBird { player, play_bird }, score))
        }
        None
    }

    fn evaluate_initial_gain_food(
        &self, 
        evaluator: &mut impl Evaluator,
        feeder: &mut impl Feeder,
        rng: &mut impl RngExt,
        discard_bird: bool,
    ) -> Option<(InitialOption, f32)> {
        let mut player = self.clone();
        let discard_bird = 
            if discard_bird { Some(player.choose_bird_to_discard(evaluator, false)?) } 
            else { None };
        let food_gained = (self.forest.len() / 2) + 1 + discard_bird.is_some() as usize;
        let mut samples = Vec::<Player>::with_capacity(LOOKAHEAD_SAMPLES);
        for _ in 0..LOOKAHEAD_SAMPLES {
            let mut p = player.clone();
            let mut d = feeder.contents();
            for _ in 0..food_gained {
                let choice = loop {
                    let matching: Vec<Food> = FOODS.into_iter().filter(|&f| d.contains(f)).collect();
                    let can_randomize = matching.len() < 2;
                    let constraint = if can_randomize {
                        FoodCollection::all()
                    } else {
                        matching.iter().fold(FoodCollection::new(), |c, &f| c + f)
                    };
                    let choice = p.choose_food_to_draw(evaluator, constraint, false);
                    if !can_randomize || d.contains(choice) {
                        break choice;
                    }
                    d = Dice::random(rng, 5);
                };
                p.food += choice;
                d.remove(d.die_from_food(choice));
            }
            samples.push(p);
        }
        let score = evaluator.evaluate_all(&samples).iter().sum::<f32>() / LOOKAHEAD_SAMPLES as f32;
        Some((InitialOption::GainFood { player, discard_bird, food_gained }, score))
    }

    fn evaluate_initial_lay_eggs(
        &self, 
        evaluator: &mut impl Evaluator,
        discard_food: bool,
    ) -> Option<(InitialOption, f32)> {
        let mut player = self.clone();
        let discard_food = 
            if discard_food { Some(player.choose_food_to_discard(evaluator, false)?) } 
            else { None };
        let eggs_laid = (player.grassland.len() / 2) + 2 + discard_food.is_some() as usize;
        let mut birds = Vec::<&'static Bird>::new();
        for _ in 0..eggs_laid {
            let possible_birds: Vec<&'static Bird> = player.bird_slots()
                .filter(|&s| s.can_fit_egg())
                .map(|s| s.bird)
                .collect();
            if let Some(bird) = player.choose_lay_egg(
                evaluator, 
                &possible_birds, 
                false
            ) { birds.push(bird) }
        }
        if birds.is_empty() { return None; }
        let score = evaluator.evaluate(&player);
        Some((InitialOption::LayEggs { player, discard_food, birds }, score))
    }

    fn evaluate_initial_draw_birds(
        &self, 
        evaluator: &mut impl Evaluator,
        bird_dealer: &mut impl BirdDealer,
        rng: &mut impl RngExt,
        discard_egg: bool,
    ) -> Option<(InitialOption, f32)> {
        let mut player = self.clone();
        let discard_egg = 
            if discard_egg { Some(player.choose_egg_to_discard(evaluator, false)?) } 
            else { None };
        let birds_drawn = (player.wetland.len() / 2) + 1 + discard_egg.is_some() as usize;
        let samples: Vec<Player> = (0..LOOKAHEAD_SAMPLES)
            .map(|_| {
                let mut p = player.clone();
                p.hand.extend(
                    bird_dealer.deck().sample(rng, birds_drawn)
                );
                p
            })
            .collect();
        let score = evaluator.evaluate_all(&samples).iter().sum::<f32>() / LOOKAHEAD_SAMPLES as f32;
        Some((InitialOption::DrawBirds { player, discard_egg, birds_drawn }, score))
    }

    fn run_when_activated(
        &mut self,
        evaluator: &mut impl Evaluator,
        feeder: &mut impl Feeder,
        bird_dealer: &mut impl BirdDealer,
        bonus_dealer: &mut impl BonusDealer,
        logged: bool,
    ) -> Vec<Action> {
        let next = |habitat: Habitat, i: usize| 
            if i != 0 { Position::Chain((habitat, i - 1)) } 
            else { Position::Triggers };
        let mut actions = Vec::<Action>::new();
        while let Position::Chain((habitat, i)) = self.activating {
            let active_bird = self.habitat(habitat)[i].bird;
            let Activation::WhenActivated(ref power) = active_bird.power else {
                self.activating = next(habitat, i);
                continue;
            }; 
            if let Some(a) = self.run_power(evaluator, feeder, bird_dealer, bonus_dealer, power) {
                let mut bird_actions = vec![Action::Activate(active_bird)];
                if logged { println!("{}", bird_actions[0]); }
                bird_actions.extend(a);
                if logged { for action in bird_actions.iter().skip(1) { println!("{}", action); } }
                actions.extend(bird_actions);
            }
            self.activating = next(habitat, i);
        }
        actions
    }

    fn run_when_played(
        &mut self,
        evaluator: &mut impl Evaluator,
        feeder: &mut impl Feeder,
        bird_dealer: &mut impl BirdDealer,
        bonus_dealer: &mut impl BonusDealer,
        logged: bool,
    ) -> Vec<Action> {
        let mut actions = Vec::<Action>::new();    
        while let Position::Played(habitat) = self.activating {
            let active_bird = self
                .habitat(habitat)
                .last()
                .expect("No bird played in given habitat")
                .bird; 
            let Activation::WhenPlayed(ref power) = active_bird.power else {
                self.activating = Position::Triggers;
                continue;
            }; 
            let Some(a) = self.run_power(
                evaluator, feeder, bird_dealer, bonus_dealer, power
            ) else {
                self.activating = Position::Triggers;
                continue;
            }; 
            let mut bird_actions = vec![Action::Activate(active_bird)]; 
            if logged { println!("{}", bird_actions[0]); }
            self.activating = match a.iter().find_map(|action| match action {
                Action::PlayBird(pb) => Some(pb.clone()),
                _ => None,
            }) {
                Some(pb) => Position::Played(pb.habitat),
                None => Position::Triggers,
            }; 
            bird_actions.extend(a);
            if logged { for action in bird_actions.iter().skip(1) { println!("{}", action); } }
            actions.extend(bird_actions);
        } 
        actions
    }
    
    // HACK: if a power can run it will
    fn run_power(
        &mut self,
        evaluator: &mut impl Evaluator,
        feeder: &mut impl Feeder,
        bird_dealer: &mut impl BirdDealer,
        bonus_dealer: &mut impl BonusDealer,
        power: &Power,
    ) -> Option<Vec<Action>> {
        let mut actions = Vec::<Action>::new();
        if let Some(habitat) = power.fewest_birds {
            if !self.fewest_birds.contains(&habitat) { return None; }
        }
        if power.hunt_dice {
            if !feeder.hunt().contains(power.food_constraint.into()) { return None; }
            actions.push(Action::HuntSuccess);
        }
        if let Some(wingspan) = power.hunt_bird {
            let b = bird_dealer.draw_from_deck();
            bird_dealer.discard(b);
            if !(b.wingspan < wingspan) { return None; }
            actions.push(Action::HuntSuccess);
        }
        if power.discard_egg { 
            actions.push(Action::DiscardEgg(
                self.choose_egg_to_discard(evaluator, true)? 
            ));
        }
        if power.discard_food {
            match power.food_constraint.into() {
                Food::Wild => actions.push(Action::DiscardFood(
                    self.choose_food_to_discard(evaluator, true)?
                )),
                food => if self.food.contains(food.into()) { 
                    self.food -= food;
                    actions.push(Action::DiscardFood(food));
                } else { return None },
            }
        }
        if power.discard_bird && (power.tuck_or_cache > 0) {
            actions.push(Action::DiscardBird(
                self.choose_bird_to_discard(evaluator, true)?
            ));
        }
        for _ in 0..power.draw_bird { 
            match (power.deck_constraint, power.face_up_constraint) {
                (true, true) => { 
                    actions.push(Action::DrawBird(
                        self.choose_bird_to_draw(evaluator, bird_dealer)
                    ));
                },
                (true, false) => { 
                    let bird = bird_dealer.draw_from_deck();
                    self.hand.push(bird);
                    actions.push(Action::DrawBird(bird));
                },
                (false, true) => {
                    if let Some(bird) = self.choose_bird_to_draw_from_face_up(evaluator, bird_dealer) {
                        actions.push(Action::DrawBird(bird));
                    } else { return Some(actions) }
                },
                (false, false) => (),
            }
        }
        if power.discard_bird && (power.tuck_or_cache == 0) {
            actions.push( Action::DiscardBird(
                self.choose_bird_to_discard(evaluator, true)?
            ));
        }
        for _ in 0..power.lay_eggs {
            let birds = self.egg_targets(power);
            if let Some(bird) = self.choose_lay_egg(evaluator, &birds, true) {
                actions.push(Action::LayEgg(bird));
            } else { return Some(actions); }
        }
        if power.lay_eggs_on_all {
            for bird in self.egg_targets(power) {
                self.lay_egg(bird);
                actions.push(Action::LayEgg(bird));
            }
        }
        let mut reroll = true;
        for _ in 0..power.gain_food { match (power.feeder, reroll) {
            (true, true) => {
                if let Some(food) = self.choose_from_feeder(evaluator, power.food_constraint, feeder) {
                    actions.push(Action::GainFood(food));
                } else { return Some(actions); }
                reroll = false;
            },
            (true, false) => {
                if let Some(food) = self.choose_from_feeder_no_reroll(evaluator, power.food_constraint, feeder) {
                    actions.push(Action::GainFood(food));
                } else { return Some(actions); }
                reroll = false;
            },
            _ => {
                actions.push(Action::GainFood(
                    self.choose_food_to_draw(evaluator, power.food_constraint, true)
                ));
            },
        }}
        if power.migration { 
            actions.push(Action::Migrate(
                self.choose_migration(evaluator)?
            ));
        }
        if power.play_bird != HabitatCollection::new() {
            actions.push(Action::PlayBird(
                self.choose_bird_to_play(evaluator, power.play_bird, true)?
            ));
        }
        if power.play_bird_in_this_habitat {
            let habitat = self.active_habitat().expect("No active habitat").into();
            actions.push(Action::PlayBird(
                self.choose_bird_to_play(evaluator, habitat, true)?
            ));
        }
        if power.tuck_or_cache > 0 {
            if power.feeder && !self.try_remove_from_feeder(power.food_constraint, feeder) { 
                return None;
            }
            self.accumulated_score += power.tuck_or_cache as usize;
            actions.push(Action::TuckOrCache(power.tuck_or_cache));
        }
        if let Some(niche) = power.repeat_power {
            let habitat = self.active_habitat().expect("No active habitat").into();
            let birds: Vec<&'static Bird> = self.habitat(habitat)
                .iter()
                .map(|s| s.bird)
                .filter(|&b| {
                    !ptr::eq(b, self.active_bird().unwrap()) &&
                    b.niche == niche &&
                    matches!(b.power, Activation::WhenActivated(_))
                })
                .collect();
            let Some(bird) = self.choose_repeat_power(&birds)
            else { return Some(actions) };
            actions.push(Action::Activate(bird));
            let Activation::WhenActivated(power) = bird.power
            else { unreachable!() };
            if let Some(a) = self.run_power(
                evaluator, 
                feeder, 
                bird_dealer, 
                bonus_dealer, 
                &power,
            ) { actions.extend(a); }
        }
        if power.draw_bonus {
            actions.push(Action::DrawBonusCard(
                self.choose_bonus_card(evaluator, bonus_dealer)
            ));
        }
        Some(actions)
    }

    fn choose_egg_to_discard(
        &mut self, 
        evaluator: &mut impl Evaluator, 
        played: bool,
    ) -> Option<&'static Bird> {
        let options = self.forest.iter()
            .chain(self.grassland.iter())
            .chain(self.wetland.iter())
            .filter(|s| s.eggs > 0)
            .map(|s| {
                let mut p = self.clone();
                p.remove_egg(s.bird);
                (s.bird, p)
            });
        let (bird, player) = evaluator.choose(options, played)?;
        *self = player;
        Some(bird)
    }
 
    fn choose_food_to_discard(
        &mut self, 
        evaluator: &mut impl Evaluator, 
        played: bool,
    ) -> Option<Food> {
        let options = FOODS.into_iter()
            .filter(|&f| self.food.contains(f.into()))
            .map(|f| {
                let mut p = self.clone();
                p.food -= f;
                (f, p)
            });
        let (food, player) = evaluator.choose(options, played)?;
        *self = player;
        Some(food)
    }
    
    fn choose_bird_to_discard(
        &mut self, 
        evaluator: &mut impl Evaluator,
        played: bool,
    ) -> Option<&'static Bird> {
        let options = self.hand.iter()
            .map(|&b| {
                let mut p = self.clone();
                p.remove_from_hand(b);
                (b, p)
            });
        let (bird, player) = evaluator.choose(options, played)?;
        *self = player;
        Some(bird)
    }

    fn choose_bird_payment(
        &mut self,
        evaluator: &mut impl Evaluator,
        cost: Cost,
        played: bool,
    ) -> FoodCollection {
        // HACK: ignores spending 2 foods as a wild
        let options: Vec<(FoodCollection, Player)> = cost.payment_options()
            .into_iter()
            .filter(|&option| self.food.contains(option))
            .map(|cost| {
                let mut p = self.clone();
                let wilds = cost.wilds;
                let mut fixed = cost;
                fixed.wilds = 0;
                p.food -= fixed;
                let mut collection = fixed;
                for _ in 0..wilds {
                    collection += p.choose_food_to_discard(evaluator, false).unwrap();
                }
                (collection, p)
            })
            .collect();
        let (collection, player) = evaluator.choose(options.into_iter(), played).unwrap();
        *self = player;
        collection
    }

    fn choose_bird_to_play(
        &mut self, evaluator: 
        &mut impl Evaluator, 
        habitat: HabitatCollection,
        played: bool,
    ) -> Option<PlayBird> {
        let options: Vec<(PlayBird, Player)> = HABITATS.into_iter()
            .filter(|h| match h {
                Habitat::Forest => habitat.forest,
                Habitat::Grassland => habitat.grassland,
                Habitat::Wetland => habitat.wetland,
            })
            .flat_map(|h| {
                self.playable_birds_in_habitat(h)
                    .into_iter()
                    .map(move |b| (b, h))
            })
            .map(|(b, h)|{
                let eggs_needed = [0, 1, 1, 2, 2][match h {
                    Habitat::Forest => self.forest.len(),
                    Habitat::Grassland => self.grassland.len(),
                    Habitat::Wetland => self.wetland.len(),
                }];
                let mut p = self.clone();
                p.play_bird(b, h);
                let mut e = Vec::<&'static Bird>::new();
                for _ in 0..eggs_needed { 
                    e.push(p.choose_egg_to_discard(evaluator, false).unwrap());
                }
                // Every candidate is evaluated; only the one chosen below is played
                let f = p.choose_bird_payment(evaluator, b.cost, false);
                (PlayBird { bird: b, food: f, eggs: e, habitat: h }, p)
            })
            .collect();
        let (action, player) = evaluator.choose(options.into_iter(), played)?;
        *self = player;
        Some(action)
    }
     
    fn choose_lay_egg(
        &mut self, 
        evaluator: &mut impl Evaluator,
        birds: &Vec<&'static Bird>,
        played: bool,
    ) -> Option<&'static Bird> {
        let options = birds.iter()
            .map(|&b| {
                let mut p = self.clone();
                p.lay_egg(b);
                (b, p)
            });
        let (bird, player) = evaluator.choose(options, played)?;
        *self = player;
        Some(bird)
    }
    
    fn choose_migration(
        &mut self, 
        evaluator: &mut impl Evaluator,
    ) -> Option<Habitat> {
        let habitat = self.active_habitat().expect("No active habitat");
        let options = HABITATS.into_iter()
            .filter(|&h| h != habitat)
            .filter(|&h| self.habitat(h).len() < 5)
            .map(|h| {
                let mut p = self.clone();
                let bird = p.habitat_mut(habitat).pop().expect("Bird is not activating");
                p.habitat_mut(h).push(bird);
                (h, p)
            });
        let (habitat, player) = evaluator.choose(options, true)?;
        *self = player;
        Some(habitat)
    }
     
    fn choose_bird_to_draw(
        &mut self,
        evaluator: &mut impl Evaluator,
        bird_dealer: &mut impl BirdDealer,
    ) -> &'static Bird {
        let draw = |bird: &'static Bird| -> Player {
            let mut p = self.clone();
            p.hand.push(bird);
            p
        };
        let deck_players: Vec<Player> = bird_dealer.deck().map(|b| draw(b)).collect();
        let deck_scores = evaluator.evaluate_all(&deck_players);
        let deck_evaluation = if deck_scores.is_empty() { 0.0 } 
            else { deck_scores.iter().sum::<f32>() / deck_scores.len() as f32 };
        let face_up_birds: Vec<&'static Bird> = bird_dealer.face_up_birds().iter().copied().collect();
        let face_up_players: Vec<Player> = face_up_birds.iter().map(|&b| draw(b)).collect();
        let face_up_scores = evaluator.evaluate_all(&face_up_players);
        let max_face_up_evaluation = face_up_birds.into_iter()
            .zip(face_up_players)
            .zip(face_up_scores)
            .map(|((b, p), score)| (b, p, score))
            .max_by(|(_, _, a), (_, _, b)| a.total_cmp(b));
        if let Some((bird, player, score)) = max_face_up_evaluation && (score > deck_evaluation) {
            *self = player;
            bird_dealer.draw_face_up(bird);
            bird
        } else {
            let bird = bird_dealer.draw_from_deck();
            *self = draw(bird);
            bird
        }
    }

    fn choose_bird_to_draw_from_face_up(
        &mut self,
        evaluator: &mut impl Evaluator,
        bird_dealer: &mut impl BirdDealer,
    ) -> Option<&'static Bird> {
        let options = bird_dealer.face_up_birds()
            .iter()
            .map(|&b| {
                let mut p = self.clone();
                p.hand.push(b);
                (b, p)
            });
        let (bird, player) = evaluator.choose(options, true)?;
        *self = player;
        bird_dealer.draw_face_up(bird);
        Some(bird)
    }
    
    fn choose_food_to_draw(
        &mut self, 
        evaluator: &mut impl Evaluator, 
        food_constraint: FoodCollection,
        played: bool,
    ) -> Food { 
        let options = FOODS.into_iter()
            .filter(|&f| food_constraint.contains(f.into()))
            .map(|f| {
                let mut p = self.clone();
                p.food += f;
                (f, p)
            });
        let (food, player) = evaluator.choose(options, played).unwrap();
        *self = player;
        food
    }

    fn choose_from_feeder(
        &mut self,
        evaluator: &mut impl Evaluator,
        food_constraint: FoodCollection,
        feeder: &mut impl Feeder,
    ) -> Option<Food> { loop {
        let contents = feeder.contents();
        let candidates: Vec<Food> = if feeder.can_randomize() {
            let f: Vec<Food> = FOODS
                .into_iter()
                .filter(|&f| food_constraint.contains(f.into()))
                .collect();
            if f.len() == 0 { feeder.randomize(); continue; } 
            f
        } else {
            FOODS.into_iter()
                .filter(|&f| food_constraint.contains(f.into()))
                .filter(|&f| contents.contains(f))
                .collect()
        };
        let options = candidates.into_iter().map(|f| {
            let mut p = self.clone();
            p.food += f;
            (f, p)
        });
        let (food, player) = evaluator.choose(options, true)?; 
        if contents.contains(food) {
            *self = player;
            feeder.take_die(contents.die_from_food(food));
            return Some(food);
        }
        feeder.randomize();
    }}

    fn choose_from_feeder_no_reroll(
        &mut self,
        evaluator: &mut impl Evaluator,
        food_constraint: FoodCollection,
        feeder: &mut impl Feeder,
    ) -> Option<Food> { loop {
        let contents = feeder.contents();
        let options = FOODS.into_iter()
            .filter(|&f| food_constraint.contains(f.into()))
            .filter(|&f| contents.contains(f))
            .map(|f| {
                let mut p = self.clone();
                p.food += f;
                (f, p)
            }
        );
        let (food, player) = evaluator.choose(options, true)?; 
        *self = player;
        feeder.take_die(contents.die_from_food(food));
        return Some(food);
    }}

    fn try_remove_from_feeder(
        &self,
        food_constraint: FoodCollection,
        feeder: &mut impl Feeder,
    ) -> bool { loop {
        let contents = feeder.contents();
        let candidates: Vec<Food> = if feeder.can_randomize() {
            FOODS.into_iter()
                .filter(|&f| food_constraint.contains(f.into()))
                .collect()
        } else {
            FOODS.into_iter()
                .filter(|&f| food_constraint.contains(f.into()))
                .filter(|&f| contents.contains(f))
                .collect()
        }; 
        for food in candidates {
            if contents.contains(food) {
                feeder.take_die(contents.die_from_food(food));
                return true;
            } 
        }
        if !feeder.can_randomize() { return false; }
        feeder.randomize();
    }}

    fn choose_repeat_power(
        &mut self,
        birds: &Vec<&'static Bird>,
    ) -> Option<&'static Bird> {
        // HACK: does not evaluate options
        birds.first().copied()
    }

    fn choose_bonus_card(
        &mut self, 
        evaluator: &mut impl Evaluator, 
        bonus_dealer: &mut impl BonusDealer,
    ) -> &'static BonusCard {
        let bonus_0 = bonus_dealer.draw();
        let bonus_1 = bonus_dealer.draw();
        let options = [(bonus_0, bonus_1), (bonus_1, bonus_0)]
            .into_iter()
            .map(|(b, d)| {
                let mut p = self.clone();
                p.bonus_cards.push(b);
                ((b, d), p)
            });
        let ((bonus_card, discard), player) = evaluator.choose(options, true).unwrap();
        bonus_dealer.discard(discard);
        *self = player;
        bonus_card
    }
}


enum InitialOption {
    PlayBird { player: Player, play_bird: PlayBird },
    LayEggs { player: Player, discard_food: Option<Food>, birds: Vec<&'static Bird> },
    GainFood { player: Player, discard_bird: Option<&'static Bird>, food_gained: usize },
    DrawBirds { player: Player, discard_egg: Option<&'static Bird>, birds_drawn: usize },
} 

impl InitialOption {
    fn activation(&self) -> Position {
        match self {
            InitialOption::PlayBird { player:_, play_bird } => 
                Position::Played(play_bird.habitat),
            InitialOption::GainFood { player, discard_bird:_, food_gained:_ } => {
                let i = player.forest.len();
                if i == 0 { Position::Triggers }
                else { Position::Chain((Habitat::Forest, i - 1)) }
            }
            InitialOption::LayEggs { player, discard_food:_, birds:_ } => {
                let i = player.grassland.len();
                if i == 0 { Position::Triggers }
                else { Position::Chain((Habitat::Grassland, i - 1)) }
            } 
            InitialOption::DrawBirds { player, discard_egg:_, birds_drawn:_ } => {
                let i = player.wetland.len();
                if i == 0 { Position::Triggers }
                else { Position::Chain((Habitat::Wetland, i - 1)) }
            }
        }
    }
}

