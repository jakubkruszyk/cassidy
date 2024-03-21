use clap::Parser;
use rand::prelude::*;
use rand::SeedableRng;
use rayon::prelude::*;
use std::iter::zip;
use std::path::PathBuf;

use crate::basestation::BaseStationEvent;
use crate::basestation::BaseStationResult;
use crate::logger::Logger;
use crate::user::User;
use crate::{
    basestation::BaseStation,
    config::{Cli, Config},
};

#[derive(Debug)]
pub struct SimState {
    pub time: u64,
    pub next_user_id: usize,
    pub lambda: f64,
    pub lambda_update_time: u64,
    pub lambda_update_idx: usize,
    pub all_users: usize,
    pub redirected_users: usize,
    pub dropped_users: usize,
}

impl SimState {
    pub fn new(cfg: &Config) -> SimState {
        let idx: usize = if cfg.lambda_coefs.len() > 1 { 1 } else { 0 };
        SimState {
            time: 0,
            next_user_id: 0,
            lambda: cfg.lambda * cfg.lambda_coefs[0].coef,
            lambda_update_time: (cfg.lambda_coefs[0].time * 3600.0 * 1000.0) as u64,
            lambda_update_idx: idx,
            all_users: 0,
            redirected_users: 0,
            dropped_users: 0,
        }
    }
}

#[derive(Debug)]
pub struct SimResults {
    pub average_usage: f64,
    pub average_power: f64,
    pub average_drop_rate: f64,
    pub stations: Vec<BaseStationResult>,
}

impl SimResults {
    pub fn new_zero(cfg: &Config) -> SimResults {
        let mut res = SimResults {
            average_usage: 0.0,
            average_power: 0.0,
            average_drop_rate: 0.0,
            stations: Vec::new(),
        };
        for _ in 0..cfg.stations_count {
            res.stations.push(BaseStationResult {
                average_power: 0.0,
                average_usage: 0.0,
                average_sleep_time: 0.0,
            })
        }
        res
    }

    pub fn add(&mut self, x: &SimResults) {
        self.average_usage += x.average_usage;
        self.average_power += x.average_power;
        self.average_drop_rate += x.average_drop_rate;
        for (s, partial) in zip(self.stations.iter_mut(), x.stations.iter()) {
            s.average_power += partial.average_power;
            s.average_usage += partial.average_usage;
            s.average_sleep_time += partial.average_sleep_time;
        }
    }
    pub fn div(&mut self, x: f64) {
        self.average_usage /= x;
        self.average_power /= x;
        self.average_drop_rate /= x;
        for s in self.stations.iter_mut() {
            s.average_power /= x;
            s.average_usage /= x;
            s.average_sleep_time /= x;
        }
    }

    fn pad(s: String, n: usize) -> String {
        format!("{:^n$}", s)
    }

    pub fn get_report(&self) -> String {
        let mut msg = format!(
            "Simulation results:\n\
            - average resource usage: {:.2} %\n\
            - average power consumption: {:.2} W\n\
            - average user drop rate: {:.2} %\n\
            \n\
            Stations results:\n\
            id  | average power [W] | average usage [%] | average sleep time [%]\n\
            ----+-------------------+-------------------+-----------------------\n",
            self.average_usage, self.average_power, self.average_drop_rate
        );
        for (i, station) in self.stations.iter().enumerate() {
            msg += (format!(
                "{} | {} | {} | {}\n",
                Self::pad(format!("{}", i), 3),
                Self::pad(format!("{:.2}", station.average_power), 17),
                Self::pad(format!("{:.2}", station.average_usage), 17),
                Self::pad(format!("{:.2}", station.average_sleep_time), 21)
            ))
            .as_str();
        }
        msg
    }
}

pub struct SimContainer {
    pub cli: Cli,
    cfg: Config,
}

impl SimContainer {
    pub fn new() -> Result<SimContainer, String> {
        let cli = Cli::parse().validate()?;
        let mut cfg = cli.create_config()?.validate()?;
        // convert lambda timestamps from hours to miliseconds
        for p in cfg.lambda_coefs.iter_mut() {
            p.time *= 3600.0 * 1000.0;
        }
        Ok(SimContainer { cli, cfg })
    }

    pub fn simulate(&self, iter: u32, log_path: PathBuf) -> SimResults {
        // initialize state
        let mut sim_state = SimState::new(&self.cfg);
        let mut rng = match &self.cli.seed {
            Some(seed) => StdRng::seed_from_u64(seed + iter as u64),
            None => StdRng::from_entropy(),
        };
        let mut logger = Logger::new(self.cli.log, &self.cfg, log_path)
            .expect("Internal error: failed to create log file");
        // create BaseStations
        let mut stations: Vec<BaseStation> = Vec::with_capacity(self.cfg.stations_count);
        for i in 0..self.cfg.stations_count {
            stations.push(BaseStation::new(i, &self.cfg, sim_state.lambda, &mut rng));
        }
        let end_time = (self.cli.duration * 3600.0 * 1000.0) as u64;

        // simulation loop
        while sim_state.time < end_time {
            // update lambda
            if sim_state.time >= sim_state.lambda_update_time {
                let l_next = &self.cfg.lambda_coefs[sim_state.lambda_update_idx];
                sim_state.lambda = self.cfg.lambda * l_next.coef;
                sim_state.lambda_update_time = sim_state.time + l_next.time as u64;
                sim_state.lambda_update_idx =
                    (sim_state.lambda_update_idx + 1) % self.cfg.lambda_coefs.len();
                if self.cli.log {
                    logger.log(
                        format!("Lambda updated to: {}", sim_state.lambda),
                        sim_state.time,
                        &self.cfg,
                    );
                }
            }

            // get next event
            let mut event_station: usize = 0;
            let (mut next_event_time, mut next_event) = stations[0].get_next_event();
            for (i, station) in stations[1..].iter().enumerate() {
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
            // Accumulate and exit early if next event exceeds simulation duration
            if next_event_time > end_time {
                let dt = end_time - sim_state.time;
                for station in stations.iter_mut() {
                    station.accumulate_counters(dt, &self.cfg);
                }
                break;
            }
            let dt = next_event_time - sim_state.time;
            sim_state.time = next_event_time;

            // Update accumulators
            for station in stations.iter_mut() {
                station.accumulate_counters(dt, &self.cfg);
            }
            match next_event {
                BaseStationEvent::AddUser => sim_state.all_users += 1,
                _ => (),
            }

            // execute event
            let res = stations[event_station].execute_event(
                &next_event,
                &self.cfg,
                &mut sim_state,
                &mut rng,
                &mut logger,
            );

            // handle possible redirections
            if let Some(user) = res {
                let redirected_user_id = user.id;
                let res = self.redirect(user, &mut stations);
                match res {
                    Ok(to_station_id) => {
                        sim_state.redirected_users += 1;
                        let from_station_id = stations[event_station].id;
                        if self.cli.log {
                            logger.log(
                                format!(
                                    "Redirect\tUser id: {} from Station id: {} to Station id: {}",
                                    redirected_user_id, from_station_id, to_station_id
                                ),
                                sim_state.time,
                                &self.cfg,
                            )
                        }
                    }
                    Err(_) => {
                        sim_state.dropped_users += 1;
                        if self.cli.log {
                            logger.log(
                                format!("User id: {} dropped", redirected_user_id),
                                sim_state.time,
                                &self.cfg,
                            )
                        }
                    }
                };
            }

            // check for potential power-up/down of stations
            if self.cli.enable_sleep {
                todo!()
            }
        }
        logger.flush();
        // return results
        let mut stations_results: Vec<BaseStationResult> = Vec::new();
        let mut avg_usage = 0.0;
        let mut avg_power = 0.0;
        for station in stations.iter() {
            let res = station.get_results(end_time);
            avg_usage += res.average_usage;
            avg_power += res.average_power;
            stations_results.push(res);
        }
        SimResults {
            average_usage: avg_usage / self.cfg.stations_count as f64,
            average_drop_rate: (sim_state.dropped_users as f64) / (sim_state.all_users as f64)
                * 100.0,
            average_power: avg_power / self.cfg.stations_count as f64,
            stations: stations_results,
        }
    }

    fn redirect(&self, user: User, stations: &mut Vec<BaseStation>) -> Result<usize, ()> {
        let redirect_station = stations
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

    fn get_log_path(&self) -> PathBuf {
        if self.cli.log {
            match &self.cli.log_path {
                Some(p) => p.to_owned(),
                None => PathBuf::from("sim.log"),
            }
        } else {
            PathBuf::from("sim.log")
        }
    }

    pub fn run(&mut self) -> SimResults {
        let mut sim_res = SimResults::new_zero(&self.cfg);
        let path = self.get_log_path();
        let partial_sim_res: Vec<SimResults> = (0..self.cli.iterations)
            .into_par_iter()
            .map(|i| {
                let mut log_path = path.clone();
                log_path.push(i.to_string());
                let res = self.simulate(i, log_path);
                if self.cli.show_partial_results {
                    println!("Partial result - iteration: {}", i);
                    println!("{}", res.get_report());
                }
                res
            })
            .collect();
        // average results
        for partial in partial_sim_res.iter() {
            sim_res.add(partial);
        }
        sim_res.div(self.cli.iterations as f64);
        sim_res
    }
}

// test only functions
impl SimContainer {
    pub fn new_test(s: usize, r: usize) -> SimContainer {
        let cli = Cli {
            with_config: None,
            seed: Some(1),
            log: false,
            log_path: None,
            duration: 1.0,
            iterations: 1,
            enable_sleep: false,
            save_default_config: None,
            show_partial_results: false,
        };
        let mut cfg = cli.create_config().unwrap();
        cfg.stations_count = s;
        cfg.resources_count = r;
        SimContainer { cli, cfg }
    }
}

#[cfg(test)]
mod test {
    use rand::{rngs::StdRng, SeedableRng};

    use crate::{
        basestation::{BaseStation, BaseStationEvent},
        logger::Logger,
        sim_container::{SimContainer, SimState},
        user::User,
    };
    use std::{io::Write, path::PathBuf, process::Command};

    #[test]
    fn redirect() {
        // test user redirection
        let container = SimContainer::new_test(3, 2);
        let mut sim_state = SimState::new(&container.cfg);
        let mut stations: Vec<BaseStation> = Vec::new();
        let mut rng = StdRng::seed_from_u64(1);
        let mut logger =
            Logger::new(false, &container.cfg, PathBuf::from("tests/redirect.log")).unwrap();
        for i in 0..container.cfg.stations_count {
            stations.push(BaseStation::new(i, &container.cfg, 1.0, &mut rng));
        }
        stations[0].execute_event(
            &BaseStationEvent::AddUser,
            &container.cfg,
            &mut sim_state,
            &mut rng,
            &mut logger,
        );
        stations[0].execute_event(
            &BaseStationEvent::AddUser,
            &container.cfg,
            &mut sim_state,
            &mut rng,
            &mut logger,
        );
        stations[2].execute_event(
            &BaseStationEvent::AddUser,
            &container.cfg,
            &mut sim_state,
            &mut rng,
            &mut logger,
        );
        // 2 redirection candidates with different usage
        let user = User {
            id: 1,
            start: 0,
            end: 10,
        };
        let res = container.redirect(user, &mut stations);
        assert_eq!(res.is_ok(), true);
        assert_eq!(res.unwrap(), 1);
        // 2 redirection candidates with same usage
        let user = User {
            id: 2,
            start: 0,
            end: 10,
        };
        let res = container.redirect(user, &mut stations);
        assert_eq!(res.is_ok(), true);
        assert_eq!(res.unwrap(), 1);
        // single redirection candidate
        let user = User {
            id: 3,
            start: 0,
            end: 10,
        };
        let res = container.redirect(user, &mut stations);
        assert_eq!(res.is_ok(), true);
        assert_eq!(res.unwrap(), 2);
        stations[2].execute_event(
            &BaseStationEvent::AddUser,
            &container.cfg,
            &mut sim_state,
            &mut rng,
            &mut logger,
        );
        // no redirection candidates
        let user = User {
            id: 3,
            start: 0,
            end: 10,
        };
        let res = container.redirect(user, &mut stations);
        assert_eq!(res.is_err(), true);
    }

    #[test]
    fn single_sim() {
        // test single simulation run
        let mut container = SimContainer::new_test(3, 10);
        container.cli.duration = 1.0 / 3600.0 * 10.0;
        let res = container.simulate(0, PathBuf::from("tests/single_sim.log"));
        let mut file =
            std::fs::File::create("tests/single_sim.report").expect("Couldn't create report file.");
        file.write(res.get_report().as_bytes())
            .expect("Couldn;t write report to file");
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
