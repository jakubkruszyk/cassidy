use std::fmt::Display;

use crate::config::Config;
use rand::{self, rngs::StdRng, Rng};

#[derive(Debug)]
pub enum UserState {
    Processed,
    Dropped,
    Redirected,
}

#[derive(Debug)]
pub struct User {
    pub id: usize,
    pub start: f64,
    pub end: f64,
    pub state: UserState,
}

impl User {
    pub fn new(id: usize, curr_time: f64, generator: &mut StdRng, cfg: &Config) -> User {
        let delay = cfg.process_time_min
            + generator.gen::<f64>() * (cfg.process_time_max - cfg.process_time_min);
        User {
            id,
            start: curr_time,
            end: curr_time + delay,
            state: UserState::Processed, // Initial state do not matter
        }
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "User id: {}, process time: {}",
            self.id,
            (self.end - self.start)
        )
    }
}
