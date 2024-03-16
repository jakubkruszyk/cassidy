use std::path::PathBuf;

use clap::Parser;
use rand::prelude::*;
use rand::SeedableRng;

use crate::logger::Logger;
use crate::user::User;
use crate::{
    basestation::BaseStation,
    config::{Cli, Config},
};

pub struct SimState {
    pub time: f64,
    pub next_user_id: usize,
    pub lambda: f64,
    pub lambda_update_time: f64,
    pub lambda_update_idx: usize,
    pub redirected_users: usize,
    pub dropped_users: usize,
}

impl SimState {
    pub fn new(cfg: &Config) -> SimState {
        let idx: usize = if cfg.lambda_coefs.len() > 1 { 1 } else { 0 };
        SimState {
            time: 0.0,
            next_user_id: 0,
            lambda: cfg.lambda * cfg.lambda_coefs[0].coef,
            lambda_update_time: cfg.lambda_coefs[0].time,
            lambda_update_idx: idx,
            redirected_users: 0,
            dropped_users: 0,
        }
    }
}

pub struct SimContainer {
    cli: Cli,
    cfg: Config,
    logger: Logger,
    rng: StdRng,
    stations: Vec<BaseStation>,
}

impl SimContainer {
    pub fn new() -> Result<SimContainer, String> {
        let cli = Cli::parse().validate()?;
        let mut cfg = cli.create_config()?.validate()?;
        let log_path = if cli.log {
            match &cli.log_path {
                Some(p) => p.to_str().unwrap(),
                None => "sim.log",
            }
        } else {
            "sim.log"
        };
        let logger = Logger::new(cli.log, &cfg, log_path)?;
        let curr_lambda = cfg.lambda * cfg.lambda_coefs[0].coef;
        // conver lambda timestamps from hours to seconds
        for p in cfg.lambda_coefs.iter_mut() {
            p.time *= 3600.0;
        }
        let mut rng = match cli.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_entropy(),
        };
        let mut stations: Vec<BaseStation> = Vec::with_capacity(cfg.stations_count);
        for i in 0..cfg.resources_count {
            stations.push(BaseStation::new(i, &cfg, curr_lambda, &mut rng));
        }
        Ok(SimContainer {
            cli,
            cfg,
            logger,
            rng,
            stations,
        })
    }

    pub fn simulate(&mut self) {
        let mut sim_state = SimState::new(&self.cfg);
        let end_time = self.cli.duration * 3600.0;
        while sim_state.time < end_time {
            // update lambda
            if sim_state.time >= sim_state.lambda_update_time {
                let l_next = &self.cfg.lambda_coefs[sim_state.lambda_update_idx];
                sim_state.lambda = self.cfg.lambda * l_next.coef;
                sim_state.lambda_update_time = sim_state.time + l_next.time;
                sim_state.lambda_update_idx =
                    (sim_state.lambda_update_idx + 1) % self.cfg.lambda_coefs.len();
                self.logger.log(
                    format!("Lambda updated to: {}", sim_state.lambda),
                    sim_state.time,
                    &self.cfg,
                );
            }

            // get next event
            let mut event_station: usize = 0;
            let (mut next_event_time, mut next_event) = self.stations[0].get_next_event();
            for (i, station) in self.stations[1..].iter().enumerate() {
                let (time, event) = station.get_next_event();
                if time < next_event_time {
                    next_event_time = time;
                    next_event = event;
                    event_station = i + 1; // offset from iteration range
                }
            }

            // update time counter
            if next_event_time < sim_state.time {
                panic!("Internal error: next event timestamp < current timestamp");
            }
            sim_state.time = next_event_time;
            if sim_state.time > end_time {
                break;
            }

            // execute event
            let res = self.stations[event_station].execute_event(
                &next_event,
                &self.cfg,
                &mut sim_state,
                &mut self.rng,
                &mut self.logger,
            );

            // handle possible redirections
            if let Some(user) = res {
                let redirected_user_id = user.id;
                let res = self.redirect(user);
                match res {
                    Ok(to_station_id) => {
                        sim_state.redirected_users += 1;
                        let from_station_id = self.stations[event_station].id;
                        self.logger.log(
                            format!(
                                "Redirect\tUser id: {} from Station id: {} to Station id: {}",
                                redirected_user_id, from_station_id, to_station_id
                            ),
                            sim_state.time,
                            &self.cfg,
                        )
                    }
                    Err(_) => {
                        sim_state.dropped_users += 1;
                        self.logger.log(
                            format!("User id: {} dropped", redirected_user_id),
                            sim_state.time,
                            &self.cfg,
                        )
                    }
                };
            }

            // check for potential power-up/down of stations
        }
        self.logger.flush();
    }

    fn redirect(&mut self, user: User) -> Result<usize, ()> {
        let redirect_station = self
            .stations
            .iter_mut()
            .min_by(|x, y| {
                x.get_usage(&self.cfg)
                    .partial_cmp(&y.get_usage(&self.cfg))
                    .unwrap()
            })
            .unwrap();
        redirect_station.redirect_here(&self.cfg, user)?;
        Ok(redirect_station.id)
    }
}

// test only functions
impl SimContainer {
    pub fn new_test(s: usize, r: usize, log: bool, log_path: PathBuf) -> SimContainer {
        let cli = Cli {
            with_config: None,
            seed: Some(1),
            log,
            log_path: Some(log_path.clone()),
            duration: 1.0,
            iterations: 1,
        };
        let mut cfg = cli.create_config().unwrap();
        cfg.stations_count = s;
        cfg.resources_count = r;
        let logger = Logger::new(log, &cfg, log_path.to_str().unwrap()).unwrap();
        let mut rng = StdRng::seed_from_u64(cli.seed.unwrap());
        let mut stations = Vec::new();
        for i in 0..s {
            stations.push(BaseStation::new(i, &cfg, 1.0, &mut rng));
        }
        SimContainer {
            cli,
            cfg,
            logger,
            rng,
            stations,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        basestation::BaseStationEvent,
        sim_container::{SimContainer, SimState},
        user::User,
    };
    use std::{path::PathBuf, process::Command};

    #[test]
    fn redirect() {
        // test user redirection
        let mut container =
            SimContainer::new_test(3, 2, false, PathBuf::from("tests/redirect.log"));
        let mut sim_state = SimState::new(&container.cfg);
        container.stations[0].execute_event(
            &BaseStationEvent::AddUser,
            &container.cfg,
            &mut sim_state,
            &mut container.rng,
            &mut container.logger,
        );
        container.stations[0].execute_event(
            &BaseStationEvent::AddUser,
            &container.cfg,
            &mut sim_state,
            &mut container.rng,
            &mut container.logger,
        );
        container.stations[2].execute_event(
            &BaseStationEvent::AddUser,
            &container.cfg,
            &mut sim_state,
            &mut container.rng,
            &mut container.logger,
        );
        // 2 redirection candidates with different usage
        let user = User {
            id: 1,
            start: 0.0,
            end: 10.0,
        };
        let res = container.redirect(user);
        assert_eq!(res.is_ok(), true);
        assert_eq!(res.unwrap(), 1);
        // 2 redirection candidates with same usage
        let user = User {
            id: 2,
            start: 0.0,
            end: 10.0,
        };
        let res = container.redirect(user);
        assert_eq!(res.is_ok(), true);
        assert_eq!(res.unwrap(), 1);
        // single redirection candidate
        let user = User {
            id: 3,
            start: 0.0,
            end: 10.0,
        };
        let res = container.redirect(user);
        assert_eq!(res.is_ok(), true);
        assert_eq!(res.unwrap(), 2);
        container.stations[2].execute_event(
            &BaseStationEvent::AddUser,
            &container.cfg,
            &mut sim_state,
            &mut container.rng,
            &mut container.logger,
        );
        // no redirection candidates
        let user = User {
            id: 3,
            start: 0.0,
            end: 10.0,
        };
        let res = container.redirect(user);
        assert_eq!(res.is_err(), true);
    }

    #[test]
    fn single_sim() {
        // test single simulation run
        let mut container =
            SimContainer::new_test(3, 10, true, PathBuf::from("tests/single_sim.log"));
        container.cli.duration = 1.0 / 3600.0 * 10.0;
        container.simulate();
        let diff = Command::new("diff")
            .args(["tests/single_sim.log", "tests/references/single_sim.log"])
            .output()
            .expect("Failed to diff results.");
        match diff.status.code() {
            Some(code) => {
                if code != 0 {
                    let _ = std::fs::write("tests/single_sim.log.diff", &diff.stdout);
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
