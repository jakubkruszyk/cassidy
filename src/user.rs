use std::fmt::Display;

use crate::config::Config;
use rand::{self, rngs::StdRng, Rng};

#[derive(Debug)]
pub struct User {
    pub id: usize,
    pub start: u64,
    pub end: u64,
}

impl User {
    pub fn new(id: usize, curr_time: u64, generator: &mut StdRng, cfg: &Config) -> User {
        // convert process_time from miliseconds to microseconds
        let delay: u64 =
            generator.gen_range((cfg.process_time_min * 1000)..=(cfg.process_time_max * 1000));
        User {
            id,
            start: curr_time,
            end: curr_time + delay,
        }
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "User id: {}, end time: {}", self.id, self.end,)
    }
}

#[cfg(test)]
mod test {
    use rand::{rngs::StdRng, SeedableRng};

    use crate::config::Config;

    use super::User;

    #[test]
    fn test_rng() {
        let cfg = Config::default();
        let mut rng = StdRng::from_entropy();
        for _ in 0..10000 {
            let user = User::new(1, 0, &mut rng, &cfg);
            assert!(
                user.end >= cfg.process_time_min * 1000 && user.end <= cfg.process_time_max * 1000,
                "min = {} user.end = {} max = {}",
                cfg.process_time_min,
                user.end,
                cfg.process_time_max,
            );
        }
    }
}
