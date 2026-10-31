//! A game of Wingspan with LooNN in one seat, driven one answer at a time: you
//! enter the cards and dice that come out, LooNN tells you its moves. Used by the
//! local play server (src/bin/play.rs) and the WebAssembly page (web/).
//!
//! Each step (the setup, or one of LooNN's turns) is replayed from a snapshot with
//! the answers given so far, so the engine never has to wait for input. When it asks
//! a question that hasn't been answered yet, the dealers hand it placeholders, the
//! network is skipped for the rest of that pass, and the result is thrown away; the
//! question is shown instead. Network results are cached, so replaying a step after
//! each answer only computes what is new.

use std::cell::{ Cell, RefCell };
use std::collections::hash_map::DefaultHasher;
use std::collections::{ HashMap, VecDeque };
use std::hash::{ Hash, Hasher };
use std::ptr;
use std::rc::Rc;
use rand::SeedableRng;
use rand::rngs::StdRng;
use crate::birds::BIRDS;
use crate::bonuses::BONUSES;
use crate::goals::GOALS;
use crate::core::{
    Action, Activation, Bird, BirdDealer, BonusCard, BonusDealer, Dice, Die, Evaluator, Feeder, Food,
    Goal, Habitat, Player, Power, ScoringEvent, Triggers, HABITATS,
};
use crate::describe::{ bird_json, esc, food_list };
use crate::evaluators::features::vectorize_player;

/// Turns each player takes over the game
const TOTAL_TURNS: u8 = 26;
/// Turns left when each round's goal is scored
const ROUND_ENDS: [u8; 4] = [18, 11, 5, 0];
/// Names the page uses for the die faces
pub const DIE_NAMES: [&str; 6] = ["invertebrate", "seed", "fish", "fruit", "rodent", "invertebrate/seed"];

/// The answers for the step being replayed, shared with the dealers
#[derive(Default)]
struct Answers {
    given: RefCell<Vec<Vec<String>>>,
    next: Cell<usize>,
    /// The first question without an answer, once the replay reaches one
    missing: RefCell<Option<Prompt>>,
}

impl Answers {
    /// The answer to the next question, or `None` (remembering the question) if it hasn't been answered
    fn ask(&self, prompt: Prompt) -> Option<Vec<String>> {
        if self.missing.borrow().is_some() { return None; }
        let i = self.next.get();
        self.next.set(i + 1);
        let answer = self.given.borrow().get(i).cloned();
        if answer.is_none() { *self.missing.borrow_mut() = Some(prompt); }
        answer
    }

    /// True once the replay has reached an unanswered question
    fn waiting(&self) -> bool { self.missing.borrow().is_some() }
}

/// A question for whoever is at the table, answered with one value per line
#[derive(Clone)]
pub struct Prompt {
    pub title: String,
    /// What was wrong with the last answer, if it was rejected
    pub error: Option<String>,
    pub kind: PromptKind,
}

#[derive(Clone)]
pub enum PromptKind {
    /// Lines `goal:<name>` (4), `bird:<name>` (5) and `bonus:<name>` (2)
    Setup,
    /// Optional lines `lay_egg`, `hunt`, `play_bird:<habitat>`, `gain_food:<food>`,
    /// `fewest:<habitat>` and `shared:<bird>`; `last` when the game is being scored
    Turn { last: bool },
    /// Die face names (see `DIE_NAMES`)
    Dice { min: u8, max: u8, prefill: Vec<&'static str> },
    /// Up to 3 bird names
    Tray { prefill: Vec<&'static Bird> },
    /// A bird name
    Bird,
    /// A bonus card name
    Bonus,
    /// Nothing to answer
    Over,
}

impl Prompt {
    fn new(kind: PromptKind, title: &str, error: &Option<String>) -> Self {
        Self { title: title.into(), error: error.clone(), kind }
    }

    pub fn to_json(&self) -> String {
        let (kind, extra) = match &self.kind {
            PromptKind::Setup => ("setup", String::new()),
            PromptKind::Turn { last } => ("turn", format!(",\"last\":{last}")),
            PromptKind::Dice { min, max, prefill } =>
                ("dice", format!(",\"min\":{min},\"max\":{max},\"prefill\":{}", json_list(prefill, |d| esc(d)))),
            PromptKind::Tray { prefill } =>
                ("tray", format!(",\"min\":0,\"max\":3,\"prefill\":{}", json_list(prefill, |b| esc(b.name)))),
            PromptKind::Bird => ("bird", String::new()),
            PromptKind::Bonus => ("bonus", String::new()),
            PromptKind::Over => ("over", String::new()),
        };
        let error = self.error.as_deref().map(esc).unwrap_or_else(|| "null".into());
        format!("{{\"kind\":{},\"title\":{},\"error\":{error}{extra}}}", esc(kind), esc(&self.title))
    }
}

fn json_list<T>(items: impl IntoIterator<Item = T>, f: impl Fn(T) -> String) -> String {
    format!("[{}]", items.into_iter().map(f).collect::<Vec<_>>().join(","))
}

fn find_bird(name: &str) -> Option<&'static Bird> {
    BIRDS.iter().find(|b| b.name.eq_ignore_ascii_case(name.trim()))
}

fn find_bonus(name: &str) -> Option<&'static BonusCard> {
    BONUSES.iter().find(|b| b.name.eq_ignore_ascii_case(name.trim()))
}

fn find_goal(name: &str) -> Option<&'static Goal> {
    GOALS.iter().find(|g| g.name.eq_ignore_ascii_case(name.trim()))
}

fn habitat_name(name: &str) -> Option<Habitat> {
    HABITATS.into_iter().find(|h| h.to_string() == name)
}

fn food_name(name: &str) -> Option<Food> {
    Some(match name {
        "invertebrate" => Food::Invertebrate,
        "seed" => Food::Seed,
        "fish" => Food::Fish,
        "fruit" => Food::Fruit,
        "rodent" => Food::Rodent,
        _ => return None,
    })
}

fn die_from_name(name: &str) -> Option<Die> {
    Some(match name {
        "invertebrate" => Die::Invertebrate,
        "seed" => Die::Seed,
        "fish" => Die::Fish,
        "fruit" => Die::Fruit,
        "rodent" => Die::Rodent,
        "invertebrate/seed" => Die::InvertebrateOrSeed,
        _ => return None,
    })
}

fn dice_names(d: &Dice) -> Vec<&'static str> {
    let counts = [d.invertebrates, d.seeds, d.fish, d.fruits, d.rodents, d.invertebrate_or_seeds];
    DIE_NAMES.into_iter().zip(counts).flat_map(|(name, n)| (0..n).map(move |_| name)).collect()
}

fn dice_count(d: &Dice) -> u8 { dice_names(d).len() as u8 }

/// Dice used while waiting for the real ones: one of each food, so no choice forces a reroll
fn placeholder_dice() -> Dice {
    Dice { invertebrates: 1, seeds: 1, fish: 1, fruits: 1, rodents: 1, invertebrate_or_seeds: 0 }
}

/// Asks for one card by name until the answer names a real card
fn ask_card<T>(answers: &Answers, kind: PromptKind, title: &str, find: impl Fn(&str) -> Result<T, String>) -> Option<T> {
    let mut error = None;
    loop {
        let answer = answers.ask(Prompt::new(kind.clone(), title, &error))?;
        match find(answer.first().map_or("", |s| s.as_str())) {
            Ok(card) => return Some(card),
            Err(problem) => error = Some(problem),
        }
    }
}


/// The birdfeeder on the table. Its dice are asked for at the start of each of
/// LooNN's turns, since other players change it, then followed through the turn.
struct AskedFeeder {
    answers: Rc<Answers>,
    tray: Cell<Option<Dice>>,
    /// Dice last seen in the feeder, offered as the starting answer
    last: Option<Dice>,
}

impl AskedFeeder {
    fn ask_dice(&self, title: &str, min: u8, max: u8, prefill: Option<Dice>) -> Option<Dice> {
        let prefill = prefill.map(|d| dice_names(&d)).unwrap_or_default();
        let mut error = None;
        loop {
            let answer = self.answers.ask(Prompt::new(PromptKind::Dice { min, max, prefill: prefill.clone() }, title, &error))?;
            let mut dice = Dice::new();
            let mut unknown = None;
            for name in &answer {
                match die_from_name(name) {
                    Some(die) => dice.insert(die),
                    None => unknown = Some(name.clone()),
                }
            }
            let n = dice_count(&dice);
            error = match unknown {
                Some(name) => Some(format!("\"{name}\" is not a die face")),
                None if n < min || n > max => Some(if min == max {
                    format!("Enter exactly {min} dice")
                } else {
                    format!("Enter {min} to {max} dice")
                }),
                None => return Some(dice),
            };
        }
    }
}

impl Feeder for AskedFeeder {
    fn contents(&self) -> Dice {
        if let Some(d) = self.tray.get() { return d; }
        let d = self.ask_dice("Which dice are in the birdfeeder?", 0, 5, self.last).unwrap_or_else(placeholder_dice);
        self.tray.set(Some(d));
        d
    }

    fn take_die(&mut self, die: Die) {
        let mut d = self.contents();
        d.remove(die);
        self.tray.set(Some(d));
    }

    fn hunt(&mut self) -> Dice {
        let outside = 5 - dice_count(&self.contents());
        if outside == 0 { return Dice::new(); }
        self.ask_dice(
            &format!("LooNN is hunting: roll the {outside} dice outside the birdfeeder. What did they show?"),
            outside, outside, None,
        ).unwrap_or_default()
    }

    fn randomize(&mut self) {
        let d = self.ask_dice("LooNN rerolls the birdfeeder: roll all 5 dice. What did they show?", 5, 5, None);
        self.tray.set(Some(d.unwrap_or_else(placeholder_dice)));
    }
}


/// The bird deck and tray on the table
struct AskedBirdDeck {
    answers: Rc<Answers>,
    /// Cards entered ahead of time, dealt before asking
    queued: VecDeque<&'static Bird>,
    tray: Option<Vec<&'static Bird>>,
    /// Birds last seen in the tray, offered as the starting answer
    last_tray: Vec<&'static Bird>,
    /// Birds LooNN has, which there is only one of, so they can't come out of the deck or tray.
    /// Entering one by mistake would confuse the engine, which tells birds apart by identity.
    owned: Vec<&'static Bird>,
}

impl AskedBirdDeck {
    fn new(answers: &Rc<Answers>, player: Option<&Player>, last_tray: Vec<&'static Bird>) -> Self {
        let owned = player.map(|p| p.hand().iter().copied().chain(p.bird_slots().map(|s| s.bird)).collect()).unwrap_or_default();
        Self { answers: answers.clone(), queued: VecDeque::new(), tray: None, last_tray, owned }
    }

    /// A bird by name, as long as LooNN doesn't already have it
    fn find_new_bird(&self, name: &str) -> Result<&'static Bird, String> {
        let bird = find_bird(name).ok_or(format!("\"{name}\" is not a bird"))?;
        if self.owned.iter().any(|b| ptr::eq(*b, bird)) {
            return Err(format!("LooNN already has {}, and there is only one", bird.name));
        }
        Ok(bird)
    }

    /// The tray as last seen, which the next turn offers as its starting answer
    fn seen_tray(&self) -> Vec<&'static Bird> { self.tray.clone().unwrap_or_else(|| self.last_tray.clone()) }
}

impl BirdDealer for AskedBirdDeck {
    fn deck(&self) -> impl Iterator<Item = &'static Bird> { BIRDS.iter() }

    fn draw_from_deck(&mut self) -> &'static Bird {
        if let Some(bird) = self.queued.pop_front() { return bird; }
        let title = "Draw the top card of the bird deck for LooNN. Which bird is it?";
        let bird = ask_card(&self.answers, PromptKind::Bird, title, |name| self.find_new_bird(name)).unwrap_or(&BIRDS[0]);
        self.owned.push(bird);
        bird
    }

    fn draw_face_up(&mut self, bird: &'static Bird) {
        self.owned.push(bird);
        if let Some(tray) = &mut self.tray {
            tray.retain(|b| !ptr::eq(*b, bird));
        }
    }

    fn refill_face_up_birds(&mut self) {
        if let Some(tray) = self.tray.take() { self.last_tray = tray; }
    }

    fn face_up_birds(&mut self) -> &[&'static Bird] {
        if self.tray.is_none() {
            let mut error = None;
            let tray = loop {
                let kind = PromptKind::Tray { prefill: self.last_tray.clone() };
                let Some(answer) = self.answers.ask(Prompt::new(kind, "Which birds are face up in the bird tray?", &error))
                    else { break Vec::new() };
                match answer.iter().map(|name| self.find_new_bird(name)).collect::<Result<Vec<_>, _>>() {
                    Ok(tray) if tray.len() > 3 => error = Some("The tray holds at most 3 birds".into()),
                    Ok(tray) if tray.iter().enumerate().any(|(i, b)| tray[..i].iter().any(|o| ptr::eq(*o, *b))) =>
                        error = Some("A bird can only be in the tray once".into()),
                    Ok(tray) => break tray,
                    Err(problem) => error = Some(problem),
                }
            };
            self.tray = Some(tray);
        }
        self.tray.as_deref().unwrap()
    }

    fn discard(&mut self, _bird: &'static Bird) { }
}


/// The bonus card deck on the table
struct AskedBonusDeck {
    answers: Rc<Answers>,
    /// Cards entered ahead of time, dealt before asking
    queued: VecDeque<&'static BonusCard>,
}

impl BonusDealer for AskedBonusDeck {
    fn draw(&mut self) -> &'static BonusCard {
        if let Some(bonus) = self.queued.pop_front() { return bonus; }
        ask_card(&self.answers, PromptKind::Bonus, "Draw a bonus card for LooNN. Which card is it?",
            |name| find_bonus(name).ok_or(format!("\"{name}\" is not a bonus card")))
            .unwrap_or(&BONUSES[0])
    }

    fn discard(&mut self, _bonus: &'static BonusCard) { }
}


/// Caches an evaluator's results, and stops evaluating once the replay is waiting on a question
struct Memo<E> {
    inner: E,
    answers: Rc<Answers>,
    cache: HashMap<u64, f32>,
}

impl<E: Evaluator> Memo<E> {
    fn key(player: &Player) -> u64 {
        let (board, hand) = vectorize_player(player);
        let mut h = DefaultHasher::new();
        player.accumulated_score().hash(&mut h);
        for v in board.iter().chain(hand.iter().flatten()) { v.to_bits().hash(&mut h); }
        hand.len().hash(&mut h);
        h.finish()
    }
}

impl<E: Evaluator> Evaluator for Memo<E> {
    fn evaluate(&mut self, player: &Player) -> f32 {
        self.evaluate_all(std::slice::from_ref(player))[0]
    }

    fn evaluate_all(&mut self, players: &[Player]) -> Vec<f32> {
        if self.answers.waiting() { return vec![0.0; players.len()]; }
        let keys: Vec<u64> = players.iter().map(Self::key).collect();
        let mut new_keys = Vec::new();
        let new_players: Vec<Player> = players.iter().zip(&keys)
            .filter(|&(_, k)| !self.cache.contains_key(k) && !new_keys.contains(k) && { new_keys.push(*k); true })
            .map(|(p, _)| p.clone())
            .collect();
        for (key, score) in new_keys.into_iter().zip(self.inner.evaluate_all(&new_players)) {
            self.cache.insert(key, score);
        }
        keys.iter().map(|k| self.cache[k]).collect()
    }

    fn choose<T>(&mut self, options: impl Iterator<Item = (T, Player)>, _played: bool) -> Option<(T, Player)> {
        let (choices, players): (Vec<T>, Vec<Player>) = options.unzip();
        let scores = self.evaluate_all(&players);
        scores.into_iter().zip(choices).zip(players)
            .max_by(|a, b| a.0.0.total_cmp(&b.0.0))
            .map(|((_, t), p)| (t, p))
    }
}


fn power_of(bird: &Bird) -> Option<&Power> {
    match &bird.power {
        Activation::None => None,
        Activation::WhenActivated(p) | Activation::WhenPlayed(p) | Activation::OnceBetweenTurns(_, p) => Some(p),
    }
}

/// Round (1 to 4) of the next turn, given the turns a player has left
fn round_of(turns_left: u8) -> usize {
    match turns_left { 19.. => 1, 12.. => 2, 6.. => 3, _ => 4 }
}

/// Whether any of the player's birds acts on other players' moves, which are asked about before each turn
pub fn has_between_turns_power(p: &Player) -> bool {
    p.bird_slots().any(|s| matches!(s.bird.power, Activation::OnceBetweenTurns(..)))
}

/// Whether any bird on the player's mat or in hand depends on having the fewest birds in a habitat
pub fn uses_fewest_birds(p: &Player) -> bool {
    p.bird_slots().map(|s| s.bird).chain(p.hand().iter().copied())
        .any(|b| power_of(b).is_some_and(|p| p.fewest_birds.is_some()))
}

fn player_json(p: &Player) -> String {
    let habitats = json_list(HABITATS, |h| format!("{}:{}", esc(&h.to_string()),
        json_list(p.habitat(h), |s| format!("{{\"bird\":{},\"eggs\":{}}}", esc(s.bird.name), s.eggs()))));
    let goals = json_list(p.goals(), |g| format!("{{\"name\":{},\"score\":{}}}", esc(g.name), g.score(p)));
    let fewest = json_list(HABITATS.into_iter().filter(|h| p.fewest_birds().contains(h)), |h| esc(&h.to_string()));
    let (between_turns, uses_fewest) = (has_between_turns_power(p), uses_fewest_birds(p));
    format!(
        "{{\"habitats\":{{{}}},\"food\":{},\"hand\":{},\"bonus\":{},\"goals\":{goals},\"score\":{},\"turns_left\":{},\"fewest\":{fewest},\"between_turns\":{between_turns},\"uses_fewest\":{uses_fewest}}}",
        &habitats[1..habitats.len() - 1],
        json_list(food_list(&p.food()), esc),
        json_list(p.hand(), |b| esc(b.name)),
        json_list(p.bonus_cards(), |b| format!("{{\"name\":{},\"score\":{}}}", esc(b.name), b.score(p))),
        p.accumulated_score(), p.total_remaining_turns(),
    )
}

fn action_lines(actions: &[Action]) -> Vec<String> {
    actions.iter().flat_map(|a| a.to_string().lines().map(String::from).collect::<Vec<_>>()).collect()
}

/// JSON for every card, so the page can offer and show them
pub fn catalog() -> String {
    let birds = json_list(&BIRDS, |b| format!("{}:{}", esc(b.name), bird_json(b)));
    let shared = json_list(BIRDS.iter().filter(|b| b.shared_power.is_some()), |b| esc(b.name));
    format!(
        "{{\"birds\":{{{}}},\"shared_power_birds\":{shared},\"bonuses\":{},\"goals\":{},\"dice\":{}}}",
        &birds[1..birds.len() - 1],
        json_list(&BONUSES, |b| esc(b.name)),
        json_list(&GOALS, |g| esc(g.name)),
        json_list(DIE_NAMES, esc),
    )
}

/// One group of LooNN's moves, for the log
pub struct Entry {
    pub title: String,
    pub lines: Vec<String>,
}

enum Stage { Setup, Turn, Over }

/// What a step produced, applied only when the step ran without waiting on a question
enum Outcome {
    Setup { player: Player, goals: [&'static Goal; 4], entry: Entry },
    Turn { player: Player, entries: Vec<Entry>, over: bool, feeder: Option<Dice>, tray: Vec<&'static Bird> },
}

/// A game in progress. `E` chooses LooNN's moves; a second `E` predicts its final score.
pub struct Session<E: Evaluator> {
    chooser: Memo<E>,
    predictor: E,
    answers: Rc<Answers>,
    stage: Stage,
    player: Option<Player>,
    goals: Option<[&'static Goal; 4]>,
    last_feeder: Option<Dice>,
    last_tray: Vec<&'static Bird>,
    log: Vec<Entry>,
    seed: u64,
    step: u64,
    /// Bumped with every new prompt, so an answer to an old one is ignored
    seq: u64,
    prompt: Prompt,
}

impl<E: Evaluator> Session<E> {
    /// Starts a game. `seed` drives LooNN's lookahead sampling, so a step replays the same way.
    pub fn new(chooser: E, predictor: E, seed: u64) -> Self {
        let answers = Rc::new(Answers::default());
        let mut session = Self {
            chooser: Memo { inner: chooser, answers: answers.clone(), cache: HashMap::new() },
            predictor,
            answers,
            stage: Stage::Setup,
            player: None,
            goals: None,
            last_feeder: None,
            last_tray: Vec::new(),
            log: Vec::new(),
            seed,
            step: 0,
            seq: 0,
            prompt: Prompt::new(PromptKind::Over, "", &None),
        };
        session.run();
        session
    }

    /// Abandons the game and starts a new one
    pub fn restart(&mut self, seed: u64) {
        self.answers.given.borrow_mut().clear();
        self.chooser.cache.clear();
        self.stage = Stage::Setup;
        self.player = None;
        self.goals = None;
        self.last_feeder = None;
        self.last_tray.clear();
        self.log.clear();
        self.seed = seed;
        self.step = 0;
        self.run();
    }

    /// Answers the current prompt, one value per line, and runs as far as the next one.
    /// Returns false if `seq` is not the current prompt's.
    pub fn answer(&mut self, seq: u64, answer: Vec<String>) -> bool {
        if seq != self.seq || matches!(self.stage, Stage::Over) { return false; }
        self.answers.given.borrow_mut().push(answer);
        self.run();
        true
    }

    /// The question waiting to be answered
    pub fn prompt(&self) -> &Prompt { &self.prompt }

    /// The current prompt's number, passed back with its answer
    pub fn seq(&self) -> u64 { self.seq }

    /// LooNN's board, once the setup is done
    pub fn player(&self) -> Option<&Player> { self.player.as_ref() }

    /// LooNN's moves so far, oldest first
    pub fn log(&self) -> &[Entry] { &self.log }

    /// LooNN's predicted final score (its actual score once the game is over)
    pub fn predicted(&mut self) -> Option<f32> {
        let player = self.player.as_ref()?;
        Some(match self.stage {
            Stage::Over => player.accumulated_score() as f32,
            _ => self.predictor.evaluate(player),
        })
    }

    /// JSON with the current prompt and the view of LooNN's game
    pub fn state_json(&mut self) -> String {
        let view = self.view_json();
        format!("{{\"seq\":{},\"prompt\":{},\"view\":{view}}}", self.seq, self.prompt.to_json())
    }

    fn view_json(&mut self) -> String {
        let status = match self.stage {
            Stage::Setup => "Waiting for the setup",
            Stage::Turn => "Waiting for LooNN's turn",
            Stage::Over => "Game over",
        };
        let predicted = match (self.predicted(), &self.stage) {
            (Some(score), Stage::Over) => score.to_string(),
            (Some(score), _) => format!("{score:.1}"),
            _ => "null".into(),
        };
        let player = self.player.as_ref().map(player_json).unwrap_or_else(|| "null".into());
        let log = json_list(&self.log, |e| format!("{{\"title\":{},\"lines\":{}}}", esc(&e.title), json_list(&e.lines, |l| esc(l))));
        format!("{{\"status\":{},\"player\":{player},\"predicted\":{predicted},\"log\":{log}}}", esc(status))
    }

    /// Replays steps with the answers given until one waits on a question
    fn run(&mut self) {
        loop {
            self.answers.next.set(0);
            *self.answers.missing.borrow_mut() = None;
            let outcome = match self.stage {
                Stage::Setup => self.run_setup(),
                Stage::Turn => self.run_turn(),
                Stage::Over => {
                    self.seq += 1;
                    self.prompt = Prompt::new(PromptKind::Over, "Game over", &None);
                    return;
                }
            };
            if let Some(missing) = self.answers.missing.borrow_mut().take() {
                self.seq += 1;
                self.prompt = missing;
                return;
            }
            let Some(outcome) = outcome else { unreachable!("a step stopped without a question") };
            match outcome {
                Outcome::Setup { player, goals, entry } => {
                    self.player = Some(player);
                    self.goals = Some(goals);
                    self.log.push(entry);
                    self.stage = Stage::Turn;
                }
                Outcome::Turn { player, entries, over, feeder, tray } => {
                    self.player = Some(player);
                    self.log.extend(entries);
                    if feeder.is_some() { self.last_feeder = feeder; }
                    self.last_tray = tray;
                    if over { self.stage = Stage::Over; }
                }
            }
            self.answers.given.borrow_mut().clear();
            // States from earlier steps rarely come up again
            self.chooser.cache.clear();
            self.step += 1;
        }
    }

    fn run_setup(&mut self) -> Option<Outcome> {
        let (goals, dealt_birds, dealt_bonuses) = self.ask_setup()?;
        let mut deck = AskedBirdDeck::new(&self.answers, None, Vec::new());
        deck.queued.extend(dealt_birds.iter().copied());
        let mut bonuses = AskedBonusDeck { answers: self.answers.clone(), queued: dealt_bonuses.iter().copied().collect() };
        let player = Player::new(&mut self.chooser, goals, &mut deck, &mut bonuses, false);

        let kept = |b: &'static Bird| player.hand().iter().any(|h| ptr::eq(*h, b));
        let names = |keep: bool| dealt_birds.iter().filter(|b| kept(b) == keep).map(|b| b.name).collect::<Vec<_>>().join(", ");
        let kept_bonus = player.bonus_cards()[0];
        let discarded_bonus = dealt_bonuses.iter().find(|b| !ptr::eq(**b, kept_bonus)).unwrap_or(&kept_bonus);
        let mut lines = vec![
            format!("Keep birds: {}", names(true)),
            format!("Discard birds: {}", names(false)),
            format!("Keep food: {}", food_list(&player.food()).join(", ")),
            format!("Keep bonus card {} and discard {}", kept_bonus.name, discarded_bonus.name),
        ];
        lines.retain(|l| !l.ends_with(": "));
        Some(Outcome::Setup { player, goals, entry: Entry { title: "Setup".into(), lines } })
    }

    /// Asks for the goals and LooNN's starting cards
    fn ask_setup(&self) -> Option<([&'static Goal; 4], Vec<&'static Bird>, Vec<&'static BonusCard>)> {
        let mut error = None;
        loop {
            let answer = self.answers.ask(Prompt::new(PromptKind::Setup, "Set up the game", &error))?;
            let (mut goals, mut birds, mut bonuses) = (Vec::new(), Vec::new(), Vec::new());
            let mut problems = Vec::<String>::new();
            for line in &answer {
                let Some((kind, name)) = line.split_once(':') else { continue };
                match kind {
                    "goal" => match find_goal(name) { Some(g) => goals.push(g), None => problems.push(format!("\"{name}\" is not a goal")) },
                    "bird" => match find_bird(name) { Some(b) => birds.push(b), None => problems.push(format!("\"{name}\" is not a bird")) },
                    "bonus" => match find_bonus(name) { Some(b) => bonuses.push(b), None => problems.push(format!("\"{name}\" is not a bonus card")) },
                    _ => (),
                }
            }
            if goals.len() != 4 { problems.push("Enter all 4 round goals".into()); }
            if birds.len() != 5 { problems.push("Enter all 5 dealt birds".into()); }
            if bonuses.len() != 2 { problems.push("Enter both dealt bonus cards".into()); }
            if birds.iter().enumerate().any(|(i, b)| birds[..i].iter().any(|o| ptr::eq(*o, *b))) {
                problems.push("Each bird can only be dealt once".into());
            }
            if goals.iter().enumerate().any(|(i, g)| goals[..i].iter().any(|o| ptr::eq(*o, *g))) {
                problems.push("Each goal can only be used once".into());
            }
            if problems.is_empty() {
                return Some((std::array::from_fn(|i| goals[i]), birds, bonuses));
            }
            error = Some(problems.join(". "));
        }
    }

    /// Asks what other players did since LooNN's last turn, which can trigger its powers
    fn ask_turn(&self, player: &Player) -> Option<Triggers> {
        let last_turn = player.total_remaining_turns() == 0;
        let title = if last_turn {
            "Once everyone has finished the game, score LooNN's last round and its game".to_string()
        } else {
            let turns_left = player.total_remaining_turns();
            let round = round_of(turns_left);
            let round_turns = [8, 7, 6, 5][round - 1];
            let turn = round_turns - (turns_left - ROUND_ENDS[round - 1]) + 1;
            format!("It's LooNN's turn: round {round}, turn {turn} of {round_turns}")
        };
        let mut error = None;
        loop {
            let answer = self.answers.ask(Prompt::new(PromptKind::Turn { last: last_turn }, &title, &error))?;
            let mut triggers = Triggers::new();
            let mut problems = Vec::<String>::new();
            for line in &answer {
                let (kind, value) = line.split_once(':').unwrap_or((line, ""));
                match (kind, value) {
                    ("lay_egg", _) => triggers.lay_egg = true,
                    ("hunt", _) => triggers.hunt_success = true,
                    ("play_bird", h) => match habitat_name(h) { Some(h) => triggers.play_bird.insert(h), None => problems.push(format!("\"{h}\" is not a habitat")) },
                    ("fewest", h) => match habitat_name(h) { Some(h) => triggers.fewest_birds.insert(h), None => problems.push(format!("\"{h}\" is not a habitat")) },
                    ("gain_food", f) => match food_name(f) { Some(f) => triggers.gain_food += f, None => problems.push(format!("\"{f}\" is not a food")) },
                    ("shared", b) => match find_bird(b).and_then(|b| b.shared_power) {
                        Some(power) => triggers.opposing_bird.push(power),
                        None => problems.push(format!("\"{b}\" has no power that other players share")),
                    },
                    _ => (),
                }
            }
            if problems.is_empty() { return Some(triggers); }
            error = Some(problems.join(". "));
        }
    }

    fn run_turn(&mut self) -> Option<Outcome> {
        let mut player = self.player.clone().expect("a turn before setup");
        let goals = self.goals.expect("a turn before setup");
        let mut triggers = self.ask_turn(&player)?;
        let mut feeder = AskedFeeder { answers: self.answers.clone(), tray: Cell::new(None), last: self.last_feeder };
        let mut deck = AskedBirdDeck::new(&self.answers, Some(&player), self.last_tray.clone());
        let mut bonuses = AskedBonusDeck { answers: self.answers.clone(), queued: VecDeque::new() };
        let mut rng = StdRng::seed_from_u64(self.seed ^ self.step.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut entries = Vec::new();
        loop {
            let turns_left = player.total_remaining_turns();
            let turn = TOTAL_TURNS - turns_left + 1;
            let score_before = player.accumulated_score();
            let (result, actions) = player.play_turn_recorded(
                &mut self.chooser, &mut feeder, &mut deck, &mut bonuses, &triggers, &mut rng, false,
            );
            if self.answers.waiting() { return None; }
            triggers = Triggers::new();
            let mut lines = action_lines(&actions);
            let gained = player.accumulated_score() - score_before;
            match result {
                Ok(()) => {
                    entries.push(Entry { title: format!("Turn {turn} (round {})", round_of(turns_left)), lines });
                    let feeder = feeder.tray.get();
                    return Some(Outcome::Turn { player, entries, over: false, feeder, tray: deck.seen_tray() });
                }
                Err(ScoringEvent::EndOfRound) => {
                    let round = ROUND_ENDS.iter().position(|&t| t == turns_left).unwrap_or(0);
                    lines.push(format!("Scored {gained} points for the round goal ({})", goals[round].name));
                    entries.push(Entry { title: format!("End of round {}", round + 1), lines });
                    // The tray is cleared at the end of each round
                    deck.tray = None;
                    deck.last_tray.clear();
                }
                Err(ScoringEvent::EndOfGame) => {
                    lines.push(format!("Scored {gained} points for the last round goal, eggs, and bonus cards"));
                    lines.push(format!("Final score: {} points", player.accumulated_score()));
                    entries.push(Entry { title: "End of game".into(), lines });
                    return Some(Outcome::Turn { player, entries, over: true, feeder: None, tray: Vec::new() });
                }
            }
        }
    }
}
