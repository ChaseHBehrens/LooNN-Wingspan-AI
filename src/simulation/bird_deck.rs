use rand::{ rng, RngExt, rngs::ThreadRng };
use crate::core::{ BirdDealer, Bird };
use crate::birds::BIRDS;


/// Simulates the bird deck
pub struct SimulatedBirdDeck {
    deck: Vec<&'static Bird>,
    face_up_birds: Vec<&'static Bird>,
    discard_pile: Vec<&'static Bird>,
    rng: ThreadRng,
    logged: bool,
}

impl SimulatedBirdDeck {
    pub fn new(logged: bool) -> Self {
        Self {
            deck: BIRDS.iter().collect(),
            face_up_birds: Vec::with_capacity(3),
            discard_pile: Vec::new(),
            rng: rng(),
            logged,
        }
    }
}

impl BirdDealer for SimulatedBirdDeck {
    fn deck(&self) -> impl Iterator<Item = &'static Bird> { BIRDS.iter() }

    fn draw_from_deck(&mut self) -> &'static Bird {
        if self.deck.is_empty() {
            // Shuffle the discard pile to form a new deck
            if self.logged { println!("Reshuffle {} discarded birds into the deck", self.discard_pile.len()); }
            self.deck.append(&mut self.discard_pile);
        }
        assert!(!self.deck.is_empty(), "Bird deck and discard pile are both empty");
        let i = self.rng.random_range(0..self.deck.len());
        let bird = self.deck.swap_remove(i);
        if self.logged { println!("Deal {}", bird); }
        bird
    }

    fn draw_face_up(&mut self, bird: &'static Bird) {
        let index = self.face_up_birds
            .iter()
            .position(|&b| std::ptr::eq(b, bird))
            .expect("The given bird is not a face up option.");
        self.face_up_birds.swap_remove(index);
    }

    fn refill_face_up_birds(&mut self) {
        for _ in 0..(3 - self.face_up_birds.len()) {
            let bird = if self.logged {
                self.logged = false;
                let bird = self.draw_from_deck();
                self.logged = true;
                bird
            } else { self.draw_from_deck() };
            if self.logged { println!("Deal {} into tray", bird); }
            self.face_up_birds.push(bird);
        }
    }

    fn face_up_birds(&mut self) -> &[&'static Bird] {
        &self.face_up_birds
    }

    fn discard(&mut self, bird: &'static Bird) {
        self.discard_pile.push(bird);
    }
}

