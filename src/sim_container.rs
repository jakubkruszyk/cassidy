use clap::Parser;
use rand::prelude::*;
use rand::SeedableRng;

use crate::logger::Logger;
use crate::{
    basestation::BaseStation,
    config::{Cli, Config},
};

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
        let logger = Logger::new(&cfg, log_path)?;
        let lambda = cfg.lambda * cfg.lambda_coefs[0].coef;
        let mut rng = match cli.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_entropy(),
        };
        let mut stations: Vec<BaseStation> = Vec::with_capacity(cfg.stations_count);
        for i in 0..cfg.resources_count {
            stations.push(BaseStation::new(i, &cfg, lambda, &mut rng));
        }
        Ok(SimContainer {
            cli,
            cfg,
            logger,
            rng,
            stations,
        })
    }
}
