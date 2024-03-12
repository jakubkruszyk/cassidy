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
        let cli = Cli::parse();
        let cfg = cli.create_config()?;
        let log_path = if cli.log {
            match &cli.log_path {
                Some(p) => p.as_str(),
                None => "sim.log",
            }
        } else {
            "sim.log"
        };
        let logger = Logger::new(cli.log, &cfg, log_path)?;
        let curr_lambda = cfg.lambda * cfg.lambda_coefs[0].coef;
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
        let end_time = self.cfg.sim_duration * 3600.0;
        while sim_state.time < end_time {
            // update lambda
            if sim_state.lambda_update_time >= sim_state.time {
                let l_next = &self.cfg.lambda_coefs[sim_state.lambda_update_idx];
                sim_state.lambda = self.cfg.lambda * l_next.coef;
                sim_state.lambda_update_time = sim_state.time + l_next.time;
                sim_state.lambda_update_idx =
                    (sim_state.lambda_update_idx + 1) % self.cfg.lambda_coefs.len();
            }

            // get next event
            let mut event_station: usize = 0;
            let (mut next_event_time, mut next_event) = self.stations[0].get_next_event();
            for (i, station) in self.stations[1..].iter().enumerate() {
                let (time, event) = station.get_next_event();
                if time < next_event_time {
                    next_event_time = time;
                    next_event = event;
                    event_station = i;
                }
            }

            // update time counter
            sim_state.time = next_event_time;

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
                            "{}  User id: {} was redirected from Station id: {} to Station id: {}",
                            sim_state.time, redirected_user_id, from_station_id, to_station_id 
                        ),
                            &self.cfg,
                        )
                    }
                    Err(_) => {
                        sim_state.dropped_users += 1;
                        self.logger.log(
                            format!("{}  User id: {} dropped", sim_state.time, redirected_user_id),
                            &self.cfg,
                        )
                    }
                };
            }

            // check for potential power-up/down of stations
        }
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
