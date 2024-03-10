use crate::config::Config;
use rand::{self, rngs::ThreadRng, Rng};

#[derive(Debug)]
pub enum UserState {
    Processed,
    Dropped,
    Redirected,
}

#[derive(Debug)]
pub struct User {
    pub start: f64,
    pub end: f64,
    pub state: UserState,
}

impl User {
    pub fn new(curr_time: f64, generator: &mut ThreadRng, cfg: &Config) -> User {
        let delay = cfg.process_time_min
            + generator.gen::<f64>() * (cfg.process_time_max - cfg.process_time_min);
        User {
            start: curr_time,
            end: curr_time + delay,
            state: UserState::Processed, // Initial state do not matter
        }
    }
}
