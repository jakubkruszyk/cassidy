use clap::Parser;
use rand::{rngs::ThreadRng, thread_rng};

use crate::{
    basestation::BaseStation,
    config::{Cli, Config},
};

pub struct SimContainer {
    cli: Cli,
    cfg: Config,
    rng: ThreadRng,
    stations: Vec<BaseStation>,
}

impl SimContainer {
    pub fn new() -> Result<SimContainer, String> {
        let cli = Cli::parse();
        let cfg = cli.create_config()?;
        let lambda = cfg.lambda * cfg.lambda_coefs[0].coef;
        let mut rng = thread_rng();
        let mut stations: Vec<BaseStation> = Vec::with_capacity(cfg.stations_count);
        for _ in 0..cfg.resources_count {
            stations.push(BaseStation::new(&cfg, lambda, &mut rng));
        }
        Ok(SimContainer {
            cli,
            cfg,
            rng,
            stations,
        })
    }
}
