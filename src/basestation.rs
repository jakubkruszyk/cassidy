use std::cmp::Ordering;

use crate::{
    config::Config,
    logger::Logger,
    user::{User, UserState},
};
use binary_heap_plus::{BinaryHeap, FnComparator};
use rand::rngs::StdRng;
use rand_distr::Distribution;

#[derive(Debug)]
pub enum BaseStationState {
    Active,
    Sleep,
    PowerUp,
    PowerDown,
}

#[derive(Debug)]
pub enum BaseStationEvent {
    ReleaseUser,
    AddUser,
}

#[derive(Debug)]
pub struct BaseStation {
    pub id: usize,
    resources: BinaryHeap<User, FnComparator<fn(&User, &User) -> Ordering>>,
    pub next_timestamp: f64,
    pub state: BaseStationState,
}

impl BaseStation {
    pub fn new(id: usize, cfg: &Config, lambda: f64, rng: &mut StdRng) -> BaseStation {
        BaseStation {
            id,
            resources: BinaryHeap::with_capacity_by(cfg.resources_count, |a: &User, b: &User| {
                b.end.partial_cmp(&a.end).unwrap()
            }),
            next_timestamp: BaseStation::get_new_timestamp(lambda, rng),
            state: BaseStationState::Active,
        }
    }

    fn get_new_timestamp(lambda: f64, rng: &mut StdRng) -> f64 {
        rand_distr::Exp::new(lambda).unwrap().sample(rng)
    }

    pub fn get_next_event(&self) -> (f64, BaseStationEvent) {
        match self.resources.peek() {
            Some(user) => {
                if user.end < self.next_timestamp {
                    (user.end, BaseStationEvent::ReleaseUser)
                } else {
                    (self.next_timestamp, BaseStationEvent::AddUser)
                }
            }
            None => (self.next_timestamp, BaseStationEvent::AddUser),
        }
    }

    pub fn execute_event(
        &mut self,
        event: &BaseStationEvent,
        cfg: &Config,
        time: f64,
        lambda: f64,
        rng: &mut StdRng,
        logger: &mut Logger,
        use_log: bool,
        next_user_id: &mut usize,
    ) -> Option<User> {
        self.next_timestamp = BaseStation::get_new_timestamp(lambda, rng);
        match event {
            BaseStationEvent::AddUser => {
                let mut user = User::new(*next_user_id, time, rng, cfg);
                *next_user_id += 1;
                if self.resources.len() >= cfg.resources_count {
                    // All resources are being used. Return user for redirect
                    user.state = UserState::Redirected;
                    return Some(user);
                }
                if use_log {
                    logger.log(
                        format!("{}  Station id {}: {} was added.", time, self.id, &user),
                        &cfg,
                    );
                }
                self.resources.push(user);
                return None;
            }
            BaseStationEvent::ReleaseUser => {
                // pop all finished users and optionally log this event
                let user = self
                    .resources
                    .pop()
                    .expect("Internal error: Tried to release user from empty heap.");
                if use_log {
                    logger.log(
                        format!("{}  Station id: {}: {} was released.", time, self.id, &user),
                        &cfg,
                    );
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{BaseStation, BaseStationEvent};
    use crate::{config::Config, logger::Logger};
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn test_add_release_user() {
        let event = BaseStationEvent::AddUser;
        let mut cfg = Config::default();
        cfg.resources_count = 10;
        let mut logger = Logger::new(&cfg, "test_add_release_user.log").unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        let mut user_id: usize = 0;
        let mut station = BaseStation::new(1, &cfg, 1.0, &mut rng);
        for _ in 0..10 {
            let res = station.execute_event(
                &event,
                &cfg,
                0.0,
                1.0,
                &mut rng,
                &mut logger,
                true,
                &mut user_id,
            );
            assert!(res.is_none() == true);
        }
        let res = station.execute_event(
            &event,
            &cfg,
            0.0,
            1.0,
            &mut rng,
            &mut logger,
            true,
            &mut user_id,
        );
        assert!(res.is_some() == true);
        let event = BaseStationEvent::ReleaseUser;
        for _ in 0..10 {
            let res = station.execute_event(
                &event,
                &cfg,
                0.0,
                1.0,
                &mut rng,
                &mut logger,
                true,
                &mut user_id,
            );
            assert!(res.is_none() == true);
        }
        logger.flush();
    }

    #[test]
    #[should_panic]
    fn test_release_user_panic() {
        let mut cfg = Config::default();
        cfg.resources_count = 10;
        let mut logger = Logger::new(&cfg, "test_release_user_panic.log").unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        let mut user_id: usize = 0;
        let mut station = BaseStation::new(1, &cfg, 1.0, &mut rng);
        let event = BaseStationEvent::ReleaseUser;
        let _ = station.execute_event(
            &event,
            &cfg,
            0.0,
            1.0,
            &mut rng,
            &mut logger,
            false,
            &mut user_id,
        );
    }
}
