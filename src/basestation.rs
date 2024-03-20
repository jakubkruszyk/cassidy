use std::cmp::Ordering;

use crate::{config::Config, logger::Logger, sim_container::SimState, user::User};
use binary_heap_plus::{BinaryHeap, FnComparator};
use rand::rngs::StdRng;
use rand_distr::Distribution;

#[derive(Debug)]
pub enum BaseStationState {
    Active,
    Sleep,
    PowerUp(u64),   // Station is during power-up process
    PowerDown(u64), // Station is during power-down process
}

#[derive(Debug)]
pub enum BaseStationEvent {
    ReleaseUser,
    AddUser,
    PowerUp,
    ShutDown,
}

#[derive(Debug)]
pub struct BaseStationResult {
    pub average_power: f64,
    pub average_usage: f64,
    pub average_sleep_time: f64,
}

#[derive(Debug)]
pub struct BaseStation {
    pub id: usize,
    resources: BinaryHeap<User, FnComparator<fn(&User, &User) -> Ordering>>,
    pub next_user_add: u64,
    pub state: BaseStationState,
    pub total_power: f64,
    pub total_usage: f64,
    pub sleep_time: u64,
}

impl BaseStation {
    pub fn new(id: usize, cfg: &Config, lambda: f64, rng: &mut StdRng) -> BaseStation {
        BaseStation {
            id,
            resources: BinaryHeap::with_capacity_by(cfg.resources_count, |a: &User, b: &User| {
                b.end.partial_cmp(&a.end).unwrap()
            }),
            next_user_add: BaseStation::get_new_timestamp(lambda, rng),
            state: BaseStationState::Active,
            total_power: 0.0,
            total_usage: 0.0,
            sleep_time: 0,
        }
    }

    pub fn clear(&mut self, lambda: f64, rng: &mut StdRng) {
        self.next_user_add = BaseStation::get_new_timestamp(lambda, rng);
        self.state = BaseStationState::Active;
        self.total_usage = 0.0;
        self.total_power = 0.0;
        self.sleep_time = 0;
        self.resources.clear();
    }

    fn get_new_timestamp(lambda: f64, rng: &mut StdRng) -> u64 {
        // converted from seconds to miliseconds
        (rand_distr::Exp::new(lambda).unwrap().sample(rng) * 1000.0) as u64
    }

    pub fn get_next_event(&self) -> (u64, BaseStationEvent) {
        match self.state {
            BaseStationState::Active => match self.resources.peek() {
                Some(user) => {
                    if user.end < self.next_user_add {
                        (user.end, BaseStationEvent::ReleaseUser)
                    } else {
                        (self.next_user_add, BaseStationEvent::AddUser)
                    }
                }
                None => (self.next_user_add, BaseStationEvent::AddUser),
            },
            BaseStationState::Sleep => (self.next_user_add, BaseStationEvent::AddUser),
            BaseStationState::PowerUp(timestamp) => {
                //
                if timestamp < self.next_user_add {
                    (timestamp, BaseStationEvent::PowerUp)
                } else {
                    (self.next_user_add, BaseStationEvent::AddUser)
                }
            }
            BaseStationState::PowerDown(timestamp) => {
                if timestamp < self.next_user_add {
                    (timestamp, BaseStationEvent::ShutDown)
                } else {
                    (self.next_user_add, BaseStationEvent::AddUser)
                }
            }
        }
    }

    pub fn execute_event(
        &mut self,
        event: &BaseStationEvent,
        cfg: &Config,
        sim_state: &mut SimState,
        rng: &mut StdRng,
        logger: &mut Logger,
    ) -> Option<User> {
        match event {
            BaseStationEvent::AddUser => {
                self.next_user_add =
                    sim_state.time + BaseStation::get_new_timestamp(sim_state.lambda, rng);
                let user = User::new(sim_state.next_user_id, sim_state.time, rng, cfg);
                sim_state.next_user_id += 1;
                match self.state {
                    BaseStationState::Active => {
                        if self.resources.len() >= cfg.resources_count {
                            // All resources are being used. Return user for redirect
                            logger.log(
                                format!(
                                    "UserCreated\tStation id: {}\t{}\tnext user: {}",
                                    self.id, &user, &self.next_user_add,
                                ),
                                sim_state.time,
                                &cfg,
                            );
                            return Some(user);
                        }
                        logger.log(
                            format!(
                                "UserAdd\tStation id: {}\t{}\tnext user: {}",
                                self.id, &user, &self.next_user_add
                            ),
                            sim_state.time,
                            &cfg,
                        );
                        self.resources.push(user);
                        None
                    }
                    BaseStationState::Sleep
                    | BaseStationState::PowerUp(_)
                    | BaseStationState::PowerDown(_) => Some(user),
                }
            }
            BaseStationEvent::ReleaseUser => {
                // pop all finished users
                if self.resources.len() == 0 {
                    panic!("Internal error: Tried to release user from empty heap.");
                }
                let user = self.resources.pop().unwrap();
                logger.log(
                    format!("UserRelease\tStation id: {}\t{}", self.id, &user),
                    sim_state.time,
                    &cfg,
                );
                None
            }
            BaseStationEvent::PowerUp => {
                logger.log(
                    format!("StateChange\tStation id: {}\tActive", self.id),
                    sim_state.time,
                    &cfg,
                );
                self.state = BaseStationState::Active;
                self.total_power += cfg.wakeup_power;
                None
            }
            BaseStationEvent::ShutDown => {
                logger.log(
                    format!("StateChange\tStation id: {}\tSleep", self.id),
                    sim_state.time,
                    &cfg,
                );
                self.state = BaseStationState::Sleep;
                // TODO: confirm this
                self.total_power += cfg.wakeup_power;
                None
            }
        }
    }

    pub fn pop_all_users(&mut self) -> Vec<User> {
        let mut users: Vec<User> = Vec::with_capacity(self.resources.len());
        while self.resources.len() > 0 {
            users.push(self.resources.pop().unwrap());
        }
        users
    }

    pub fn get_usage(&self, cfg: &Config) -> f64 {
        (self.resources.len() as f64) / (cfg.resources_count as f64) * 100.0
    }

    pub fn redirect_here(&mut self, cfg: &Config, user: User) -> Result<(), ()> {
        if self.resources.len() >= cfg.resources_count {
            return Err(());
        }
        self.resources.push(user);
        Ok(())
    }

    pub fn accumulate_counters(&mut self, dt: u64, cfg: &Config) {
        let dp = match self.state {
            BaseStationState::Active => dt as f64 * cfg.active_power,
            BaseStationState::Sleep => {
                self.sleep_time += dt;
                dt as f64 * cfg.sleep_power
            }
            BaseStationState::PowerUp(_) => 0.0,
            BaseStationState::PowerDown(_) => 0.0,
        };
        self.total_power += dp;
        self.total_usage += dt as f64 * self.get_usage(&cfg);
    }

    pub fn get_results(&self, total_time: u64) -> BaseStationResult {
        BaseStationResult {
            average_power: self.total_power / total_time as f64,
            average_usage: self.total_usage / total_time as f64,
            average_sleep_time: self.sleep_time as f64 / total_time as f64,
        }
    }
}

// Methods for testing only
impl BaseStation {
    fn force_add_user(&mut self, user: User) {
        self.resources.push(user);
    }
}

#[cfg(test)]
mod test {
    use super::{BaseStation, BaseStationEvent, BaseStationState};
    use crate::{config::Config, logger::Logger, sim_container::SimState, user::User};
    use rand::{rngs::StdRng, SeedableRng};
    use std::process::Command;

    #[test]
    fn add_release_user() {
        // Test adding users to max capacity and check return type
        let event = BaseStationEvent::AddUser;
        let mut cfg = Config::default();
        cfg.resources_count = 10;
        let mut logger = Logger::new(false, &cfg, "test_add_release_user.log").unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        let mut station = BaseStation::new(1, &cfg, 1.0, &mut rng);
        let mut sim_state = SimState::new(&cfg);
        sim_state.lambda = 1.0;
        sim_state.time = 0;
        for _ in 0..10 {
            let res = station.execute_event(&event, &cfg, &mut sim_state, &mut rng, &mut logger);
            assert!(res.is_none() == true);
        }
        let res = station.execute_event(&event, &cfg, &mut sim_state, &mut rng, &mut logger);
        assert!(res.is_some() == true);
        // Test releasing all users and  return type - should not panic
        let event = BaseStationEvent::ReleaseUser;
        for _ in 0..10 {
            let res = station.execute_event(&event, &cfg, &mut sim_state, &mut rng, &mut logger);
            assert!(res.is_none() == true);
        }
        logger.flush();
    }

    #[test]
    #[should_panic]
    fn release_user_panic() {
        // Test release from empty heap
        let mut cfg = Config::default();
        cfg.resources_count = 10;
        let mut logger = Logger::new(false, &cfg, "test_release_user_panic.log").unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        let mut station = BaseStation::new(1, &cfg, 1.0, &mut rng);
        let mut sim_state = SimState::new(&cfg);
        let event = BaseStationEvent::ReleaseUser;
        let _ = station.execute_event(&event, &cfg, &mut sim_state, &mut rng, &mut logger);
    }

    #[test]
    fn add_user_all_states() {
        // Adding (redirect) from states different from Active
        let mut cfg = Config::default();
        cfg.resources_count = 10;
        let mut logger = Logger::new(false, &cfg, "test_add_user_all_states.log").unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        let mut sim_state = SimState::new(&cfg);
        let mut station = BaseStation::new(1, &cfg, 1.0, &mut rng);
        // test add (redirect) during sleep state
        station.state = BaseStationState::Sleep;
        let res = station.execute_event(
            &BaseStationEvent::AddUser,
            &cfg,
            &mut sim_state,
            &mut rng,
            &mut logger,
        );
        assert!(res.is_some() == true);
        assert!(station.resources.len() == 0);
        // test add (redirect) during power-uo/down state
        station.state = BaseStationState::PowerUp(10);
        let res = station.execute_event(
            &BaseStationEvent::AddUser,
            &cfg,
            &mut sim_state,
            &mut rng,
            &mut logger,
        );
        assert!(res.is_some() == true);
        assert!(station.resources.len() == 0);
        station.state = BaseStationState::PowerDown(10);
        let res = station.execute_event(
            &BaseStationEvent::AddUser,
            &cfg,
            &mut sim_state,
            &mut rng,
            &mut logger,
        );
        assert!(res.is_some() == true);
        assert!(station.resources.len() == 0);
    }

    #[test]
    fn get_event() {
        let mut cfg = Config::default();
        cfg.resources_count = 10;
        let mut logger = Logger::new(false, &cfg, "test_add_user_all_states.log").unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        let mut sim_state = SimState::new(&cfg);
        let mut station = BaseStation::new(1, &cfg, 1.0, &mut rng);
        // test active state
        station.state = BaseStationState::Active;
        station.force_add_user(User {
            id: 1,
            start: 0,
            end: 10,
        });
        station.force_add_user(User {
            id: 2,
            start: 0,
            end: 20,
        });
        assert!(station.resources.len() == 2);
        station.next_user_add = 15;
        let res = station.get_next_event();
        assert_eq!(res.0, 10);
        assert!(std::matches!(res.1, BaseStationEvent::ReleaseUser));
        station.execute_event(
            &BaseStationEvent::ReleaseUser,
            &cfg,
            &mut sim_state,
            &mut rng,
            &mut logger,
        );
        assert_eq!(station.resources.len(), 1);
        let res = station.get_next_event();
        assert_eq!(res.0, 15);
        assert!(std::matches!(res.1, BaseStationEvent::AddUser));
        // test sleep state
        station.force_add_user(User {
            id: 1,
            start: 0,
            end: 10,
        });
        station.state = BaseStationState::Sleep;
        let res = station.get_next_event();
        assert_eq!(res.0, 15);
        assert!(std::matches!(res.1, BaseStationEvent::AddUser));

        // test power-up/down state
        station.state = BaseStationState::PowerUp(25);
        let res = station.get_next_event();
        assert_eq!(res.0, 15);
        assert!(std::matches!(res.1, BaseStationEvent::AddUser));
        station.next_user_add = 30;
        let res = station.get_next_event();
        assert_eq!(res.0, 25);
        assert!(std::matches!(res.1, BaseStationEvent::PowerUp));

        station.state = BaseStationState::PowerDown(25);
        station.next_user_add = 15;
        let res = station.get_next_event();
        assert_eq!(res.0, 15);
        assert!(std::matches!(res.1, BaseStationEvent::AddUser));
        station.next_user_add = 30;
        let res = station.get_next_event();
        assert_eq!(res.0, 25);
        assert!(std::matches!(res.1, BaseStationEvent::ShutDown));
    }

    #[test]
    fn test_add_release_order() {
        let mut cfg = Config::default();
        cfg.resources_count = 10;
        let mut logger = Logger::new(true, &cfg, "tests/add_release_order.log").unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        let mut sim_state = SimState::new(&cfg);
        let mut station = BaseStation::new(1, &cfg, 1.0, &mut rng);
        // add users to max capacity
        for _ in 0..10 {
            let res = station.execute_event(
                &BaseStationEvent::AddUser,
                &cfg,
                &mut sim_state,
                &mut rng,
                &mut logger,
            );
            assert_eq!(res.is_none(), true);
        }
        // release all users
        for _ in 0..10 {
            let res = station.execute_event(
                &BaseStationEvent::ReleaseUser,
                &cfg,
                &mut sim_state,
                &mut rng,
                &mut logger,
            );
            assert_eq!(res.is_none(), true);
        }
        logger.flush();
        // compare log to reference
        let diff = Command::new("diff")
            .args([
                "tests/add_release_order.log",
                "tests/references/add_release_order.log",
            ])
            .output()
            .expect("Failed to diff results.");
        match diff.status.code() {
            Some(code) => {
                if code != 0 {
                    let _ = std::fs::write("tests/add_release_order.log.diff", &diff.stdout);
                    panic!(
                        "error code != 0\n{}",
                        String::from_utf8(diff.stdout).unwrap()
                    );
                }
            }
            None => panic!("Unable to unwrap error code"),
        };
        assert_eq!(diff.status.code().unwrap(), 0);
    }
}
