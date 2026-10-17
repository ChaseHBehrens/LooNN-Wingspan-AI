use inquire::{ InquireError, Select };
use crate::core::{ Bird, BirdDealer };
use crate::birds::BIRDS;


/// User input for bird card deck
pub struct UserInterfaceBirdDeck {
    face_up_birds: Option<Vec<&'static Bird>>,
}

impl UserInterfaceBirdDeck {
    pub fn new() -> Self {
        Self { face_up_birds: None }
    }

    /// Gets a `&'static Bird` from the user via the terminal
    fn get_bird_input() -> Result<&'static Bird, InquireError> {
        Select::new(&format!("Enter bird name:"), BIRDS.iter().collect())
            .with_page_size(5)
            .prompt()
    }
}


impl BirdDealer for UserInterfaceBirdDeck {
    fn deck(&self) -> impl Iterator<Item = &'static Bird> { BIRDS.iter() }
    
    fn draw_from_deck(&mut self) -> &'static Bird {
        println!("Enter the name of the bird drawn from the deck:");
        loop {
            match Self::get_bird_input() {
                Ok(bird) => return bird,
                Err(_) => continue,
            }
        }
    }

    fn draw_face_up(&mut self, _bird: &'static Bird) { }

    fn refill_face_up_birds(&mut self) { self.face_up_birds = None }

    fn face_up_birds(&mut self) -> &[&'static Bird] {
        self.face_up_birds.get_or_insert_with(|| {
            let mut face_up_birds = Vec::with_capacity(3);
            for _ in 0..3 {
                match Self::get_bird_input() {
                    Ok(bird) => face_up_birds.push(bird),
                    Err(_) => return face_up_birds,
                }
            }
            face_up_birds
        })
    }

    fn discard(&mut self, _bird: &'static Bird) { }
}
