use clap::Parser;
use rand::prelude::*;
use rand::SeedableRng;
use rayon::prelude::*;
use std::ffi::OsString;
use std::io::Write;
use std::iter::zip;
use std::path::PathBuf;

use crate::basestation::{BaseStation, BaseStationEvent, BaseStationResult, BaseStationState};
use crate::config::WalkOverType;
use crate::logger::Logger;
use crate::{
    config::{Cli, Config},
    user::User,
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
            lambda_update_time: cfg.lambda_coefs[0].time as u64,
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
    pub total_users: usize,
    pub stations: Vec<BaseStationResult>,
}

impl SimResults {
    pub fn new_zero(cfg: &Config) -> SimResults {
        let mut res = SimResults {
            average_usage: 0.0,
            average_power: 0.0,
            average_drop_rate: 0.0,
            total_users: 0,
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
        self.total_users += x.total_users;
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
        self.total_users = (self.total_users as f64 / x) as usize;
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
            - processed users: {} \n\
            - average resource usage: {:.2} %\n\
            - average power consumption: {:.2} W\n\
            - average user drop rate: {:.2} %\n\
            \n\
            Stations results:\n\
            id  | average power [W] | average usage [%] | average sleep time [%]\n\
            ----+-------------------+-------------------+-----------------------\n",
            self.total_users, self.average_usage, self.average_power, self.average_drop_rate
        );
        for (i, station) in self.stations.iter().enumerate() {
            msg += (format!(
                "{} | {} | {} | {}\n",
                Self::pad(format!("{}", i), 3),
                Self::pad(format!("{:.2}", station.average_power), 17),
                Self::pad(format!("{:.2}", station.average_usage), 17),
                Self::pad(format!("{:.2}", station.average_sleep_time * 100.0), 21)
            ))
            .as_str();
        }
        msg
    }

    pub fn get_csv_header(&self) -> String {
        let mut msg = format!(
            "processed_users,average_resource_usage,average_power_consumption,average_user_drop_rate"
        );
        for i in 0..self.stations.len() {
            msg += (format!(
                ",station{}_average_power,station{}_average_usage,station{}_average_sleep_time",
                i, i, i
            ))
            .as_str();
        }
        msg
    }

    pub fn get_csv(&self) -> String {
        let mut data = format!(
            "{},{},{},{}",
            self.total_users, self.average_usage, self.average_power, self.average_drop_rate
        );
        for station in self.stations.iter() {
            data += &format!(
                ",{},{},{}",
                station.average_power, station.average_usage, station.average_sleep_time
            );
        }
        data
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
        // convert lambda timestamps from hours to microseconds
        for p in cfg.lambda_coefs.iter_mut() {
            p.time *= 3600.0 * 1000_000.0;
        }
        Ok(SimContainer { cli, cfg })
    }

    pub fn update_param(&mut self, param: &WalkOverType, value: f64) {
        match param {
            WalkOverType::Lambda => {
                self.cfg.lambda = value;
            }
            WalkOverType::SleepLow => {
                self.cfg.sleep_threshold = value;
            }
            WalkOverType::SleepHigh => {
                self.cfg.wakeup_threshold = value;
            }
        }
    }

    pub fn simulate(&self, iter: u32, log_path: PathBuf) -> SimResults {
        // initialize state
        let mut sim_state = SimState::new(&self.cfg);
        let mut rng = match &self.cli.seed {
            Some(seed) => StdRng::seed_from_u64(seed + iter as u64),
            None => StdRng::from_entropy(),
        };
        let mut logger = Logger::new(self.cli.log, &self.cfg, &log_path)
            .expect("Internal error: failed to create log file");
        // create BaseStations
        let mut stations: Vec<BaseStation> = Vec::with_capacity(self.cfg.stations_count);
        for i in 0..self.cfg.stations_count {
            stations.push(BaseStation::new(i, &self.cfg, sim_state.lambda, &mut rng));
        }
        // turn all but first station to sleep if enabled
        if self.cli.enable_sleep {
            for station in stations.iter_mut().skip(1) {
                station.state = BaseStationState::Sleep;
            }
        }
        // convert end time from hours to microseconds
        let end_time = (self.cli.duration * 3600.0 * 1000_000.0) as u64;

        // initialize binary logger
        let mut sample_counter = 0;
        let mut bin_path = log_path.clone();
        bin_path.set_file_name("sim_bin");
        bin_path.set_extension(log_path.extension().unwrap());
        let mut wave_file = if self.cli.log_wave {
            let mut wave_file = std::fs::File::create(bin_path).unwrap();
            let _ = wave_file.write(&(self.cfg.stations_count as u32).to_le_bytes());
            Some(wave_file)
        } else {
            None
        };

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
                self.cli.log,
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
                self.power_up_down(&sim_state, &mut stations);
            }

            // Binary logging
            match wave_file.as_mut() {
                Some(file) => {
                    // each "row" of data consists of:
                    // - timestamp 8 bytes
                    // - stations data:
                    //   - usage 4 bytes
                    //   - state 1 byte
                    sample_counter += 1;
                    if sample_counter >= self.cli.samples {
                        sample_counter = 0;
                        let mut data = Vec::from(sim_state.time.to_le_bytes());
                        for station in stations.iter() {
                            for byte in (station.get_usage_raw() as u32).to_le_bytes() {
                                data.push(byte)
                            }
                            let state_code: u8 = match station.state {
                                BaseStationState::Active => 4,
                                BaseStationState::Sleep => 1,
                                BaseStationState::PowerUp(_) => 3,
                                BaseStationState::PowerDown(_) => 2,
                            };
                            data.push(state_code);
                        }
                        let _ = file.write_all(&data);
                    }
                }
                None => (),
            };
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
            total_users: sim_state.all_users,
            stations: stations_results,
        }
    }

    /// Takes single user and assigns it to active station with lowest usage
    fn redirect(&self, user: User, stations: &mut Vec<BaseStation>) -> Result<usize, ()> {
        let redirect_station = stations
            .iter_mut()
            .filter(|x| match x.state {
                BaseStationState::Active => true,
                _ => false,
            })
            .min_by(|x, y| {
                x.get_usage(&self.cfg)
                    .partial_cmp(&y.get_usage(&self.cfg))
                    .unwrap()
            })
            .unwrap();
        redirect_station.redirect_here(&self.cfg, user)?;
        Ok(redirect_station.id)
    }

    fn power_up_down(&self, sim_state: &SimState, stations: &mut Vec<BaseStation>) {
        let heavy_load = stations
            .iter()
            .position(|x| x.get_usage(&self.cfg) >= self.cfg.wakeup_threshold);
        match heavy_load {
            Some(idx) => self.try_wakeup(sim_state, idx, stations),
            None => self.try_shutdown(sim_state, stations),
        };
    }

    fn try_wakeup(
        &self,
        sim_state: &SimState,
        heavy_load_idx: usize,
        stations: &mut Vec<BaseStation>,
    ) {
        // Assumption -> wakeup station is empty or has very little users registered
        // so there is need for only one redirection
        stations
            .iter()
            .position(|x| match x.state {
                BaseStationState::Sleep => true,
                _ => false,
            })
            .map(|idx| {
                // Redirect half of load to woken up station
                let mut users = stations[heavy_load_idx].release_half();
                for user in users.iter_mut() {
                    user.end += self.cfg.wakeup_delay * 1000;
                }
                stations[idx].state =
                    BaseStationState::PowerUp(sim_state.time + self.cfg.wakeup_delay * 1000);
                let u_len = users.len();
                stations[idx].redirect_here_vec(&self.cfg, &mut users, u_len);
                debug_assert!(users.len() == 0);
            });
    }

    fn try_shutdown(&self, sim_state: &SimState, stations: &mut Vec<BaseStation>) {
        // Find station with usage below sleep_threshold
        let shutdown_station_id = stations
            .iter()
            .filter(|s| s.is_active())
            .find(|s| s.get_usage(&self.cfg) <= self.cfg.sleep_threshold)
            .map(|s| s.id);

        // Early return (most likely case)
        let shutdown_station_id = match shutdown_station_id {
            Some(id) => id,
            None => return,
        };

        // Get all active stations (except shutdown candidate) id and their remaining capacity
        let mut active_capacity: usize = 0;
        let active_capacity_list: Vec<(usize, usize)> = stations
            .iter()
            .filter(|s| s.is_active() && s.id != shutdown_station_id)
            .map(|s| {
                let capacity = self.cfg.resources_count - s.get_usage_raw();
                active_capacity += capacity;
                (s.id, capacity)
            })
            .collect();

        // There always must be at least single active station,
        // so when looking for station for power down there must be
        // at least 2 active stations
        if active_capacity_list.len() < 2 {
            return;
        }

        // Check if there is enough capacity in active stations for users from shutdown station
        let station = &mut stations[shutdown_station_id];
        if station.get_usage_raw() >= active_capacity {
            return;
        }

        let mut users = station.release_all();
        station.state = BaseStationState::PowerDown(sim_state.time + self.cfg.wakeup_delay * 1000);

        // Redirect users to all other active stations, proportionally to their remaining capacity
        let u_len = users.len();
        for (idx, capacity) in &active_capacity_list[0..active_capacity_list.len() - 1] {
            let count = capacity * u_len / active_capacity;
            stations[*idx].redirect_here_vec(&self.cfg, &mut users, count);
        }
        // Redirect all remaining users to last station
        let (last_idx, _) = active_capacity_list[active_capacity_list.len() - 1];
        let u_len = users.len();
        stations[last_idx].redirect_here_vec(&self.cfg, &mut users, u_len);
        debug_assert_eq!(users.len(), 0);
    }

    pub fn run(&self, run_no: usize) -> SimResults {
        let mut sim_res = SimResults::new_zero(&self.cfg);
        let path = PathBuf::from(format!("sim.run_{}_no_", run_no));
        let partial_sim_res: Vec<SimResults> = (0..self.cli.iterations)
            .into_par_iter()
            .map(|i| {
                let mut log_path: OsString = path.clone().into();
                log_path.push(i.to_string());
                let res = self.simulate(i, log_path.into());
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
    #[allow(dead_code)]
    pub fn new_test(s: usize, r: usize) -> SimContainer {
        let cli = Cli {
            with_config: None,
            seed: Some(1),
            log: false,
            log_wave: false,
            duration: 1.0,
            iterations: 1,
            enable_sleep: false,
            save_default_config: None,
            show_partial_results: false,
            samples: 1,
            walk_over: None,
        };
        let mut cfg = cli.create_config().unwrap();
        // convert lambda timestamps from hours to microseconds
        for p in cfg.lambda_coefs.iter_mut() {
            p.time *= 3600.0 * 1000_000.0;
        }
        cfg.stations_count = s;
        cfg.resources_count = r;
        SimContainer { cli, cfg }
    }
}

#[cfg(test)]
mod test {
    use rand::{rngs::StdRng, SeedableRng};

    use crate::{
        basestation::{BaseStation, BaseStationEvent, BaseStationState},
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
            Logger::new(false, &container.cfg, &PathBuf::from("tests/redirect.log")).unwrap();
        for i in 0..container.cfg.stations_count {
            stations.push(BaseStation::new(i, &container.cfg, 1.0, &mut rng));
        }
        stations[0].execute_event(
            &BaseStationEvent::AddUser,
            &container.cfg,
            &mut sim_state,
            &mut rng,
            false,
            &mut logger,
        );
        stations[0].execute_event(
            &BaseStationEvent::AddUser,
            &container.cfg,
            &mut sim_state,
            &mut rng,
            false,
            &mut logger,
        );
        stations[2].execute_event(
            &BaseStationEvent::AddUser,
            &container.cfg,
            &mut sim_state,
            &mut rng,
            false,
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
            false,
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
        container.cli.duration = 1.0 / 3600.0 * 60.0;
        container.cli.log = true;
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

    #[test]
    fn try_wakeup() {
        let sim = SimContainer::new_test(3, 10);
        let sim_state = SimState::new(&sim.cfg);
        let mut rng = StdRng::seed_from_u64(1);
        let mut stations = vec![
            BaseStation::new(1, &sim.cfg, 1.0, &mut rng),
            BaseStation::new(1, &sim.cfg, 1.0, &mut rng),
            BaseStation::new(1, &sim.cfg, 1.0, &mut rng),
        ];
        stations[1].state = BaseStationState::Sleep;
        for i in 0..10 {
            stations[0].force_add_user(User {
                id: i,
                start: 0,
                end: i as u64,
            });
        }

        sim.try_wakeup(&sim_state, 0, &mut stations);
        // assert stations state
        assert!(std::matches!(
            stations[1].state,
            BaseStationState::PowerUp(_)
        ));
        // assert redirected users
        assert_eq!(stations[0].get_usage(&sim.cfg), 50.0);
        assert_eq!(stations[1].get_usage(&sim.cfg), 50.0);
        assert_eq!(stations[2].get_usage(&sim.cfg), 0.0);

        sim.try_wakeup(&sim_state, 0, &mut stations);
        assert!(std::matches!(stations[0].state, BaseStationState::Active));
        assert!(std::matches!(
            stations[1].state,
            BaseStationState::PowerUp(_)
        ));
        assert!(std::matches!(stations[2].state, BaseStationState::Active));
        assert_eq!(stations[0].get_usage(&sim.cfg), 50.0);
        assert_eq!(stations[1].get_usage(&sim.cfg), 50.0);
        assert_eq!(stations[2].get_usage(&sim.cfg), 0.0);
    }

    #[test]
    fn try_shutdown() {
        let sim = SimContainer::new_test(3, 20);
        let mut sim_state = SimState::new(&sim.cfg);
        let mut logger = Logger::new(false, &sim.cfg, &PathBuf::from("try_shutdown.log")).unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        let mut stations = vec![
            BaseStation::new(0, &sim.cfg, 1.0, &mut rng),
            BaseStation::new(1, &sim.cfg, 1.0, &mut rng),
            BaseStation::new(2, &sim.cfg, 1.0, &mut rng),
        ];
        for _ in 0..10 {
            stations[1].execute_event(
                &BaseStationEvent::AddUser,
                &sim.cfg,
                &mut sim_state,
                &mut rng,
                false,
                &mut logger,
            );
        }
        for _ in 0..6 {
            stations[2].execute_event(
                &BaseStationEvent::AddUser,
                &sim.cfg,
                &mut sim_state,
                &mut rng,
                false,
                &mut logger,
            );
        }
        for _ in 0..4 {
            stations[0].execute_event(
                &BaseStationEvent::AddUser,
                &sim.cfg,
                &mut sim_state,
                &mut rng,
                false,
                &mut logger,
            );
        }

        // Test shutdown
        // 4 users from station 0 redirected to:
        // - station 1: 10/24 * 4 -> 1 user
        // - station 2: 4 - 1 = 3 users
        sim.try_shutdown(&sim_state, &mut stations);
        assert!(std::matches!(
            stations[0].state,
            BaseStationState::PowerDown(_)
        ));
        assert_eq!(stations[0].get_usage_raw(), 0);
        assert!(std::matches!(stations[1].state, BaseStationState::Active));
        assert_eq!(stations[1].get_usage_raw(), 11);
        assert!(std::matches!(stations[2].state, BaseStationState::Active));
        assert_eq!(stations[2].get_usage_raw(), 9);

        // Test no shutdown when there are less than 2 active stations
        stations[1].state = BaseStationState::Sleep;
        let _ = stations[2].release_all();
        sim.try_shutdown(&sim_state, &mut stations);
        assert!(std::matches!(
            stations[0].state,
            BaseStationState::PowerDown(_)
        ));
        assert_eq!(stations[0].get_usage_raw(), 0);
        assert_eq!(stations[1].get_usage_raw(), 11);
        assert!(std::matches!(stations[2].state, BaseStationState::Active));
        assert_eq!(stations[2].get_usage_raw(), 0);

        // Test no shutdown when there is not enough space for redirected users
        stations[0].state = BaseStationState::Active;
        stations[0].execute_event(
            &BaseStationEvent::AddUser,
            &sim.cfg,
            &mut sim_state,
            &mut rng,
            false,
            &mut logger,
        );
        for _ in 0..20 {
            stations[2].execute_event(
                &BaseStationEvent::AddUser,
                &sim.cfg,
                &mut sim_state,
                &mut rng,
                false,
                &mut logger,
            );
        }
        sim.try_shutdown(&sim_state, &mut stations);
        assert!(std::matches!(stations[0].state, BaseStationState::Active));
        assert_eq!(stations[0].get_usage_raw(), 1);
        assert!(std::matches!(stations[1].state, BaseStationState::Sleep));
        assert_eq!(stations[1].get_usage_raw(), 11);
        assert!(std::matches!(stations[2].state, BaseStationState::Active));
        assert_eq!(stations[2].get_usage_raw(), 20);
    }
}
