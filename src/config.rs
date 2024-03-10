use std::{fs::File, io::Read, path::PathBuf};

use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Number of available resource blocks in BaseStation.
    #[arg(long, value_name = "number")]
    resources: Option<u32>,
    /// Path to config file. This option cannot be used with any other config switch.
    #[arg(long, value_name = "path")]
    with_config: Option<PathBuf>,
}

impl Cli {
    pub fn create_config(&self) -> Result<Config, String> {
        if let Some(file_path) = &self.with_config {
            let file = File::open(&file_path);
            let mut file = match file {
                Ok(f) => f,
                Err(_) => return Err(format!("Cannot open file: {}", file_path.display())),
            };
            let mut data = String::new();
            let res = file.read_to_string(&mut data);
            match res {
                Err(_) => return Err(format!("Cannot read file: {}", file_path.display())),
                _ => (),
            }
            match toml::from_str::<Config>(&data) {
                Ok(c) => Ok(c),
                Err(e) => Err(e.to_string()),
            }
        } else {
            Ok(Config::default())
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LambdaPoint {
    pub time: f64, // [h]
    pub coef: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub process_time_max: f64, // [s]
    pub process_time_min: f64, // [s]
    pub lambda: f64,
    pub lambda_coefs: Vec<LambdaPoint>,
    pub resources_count: usize,
    pub sleep_threshold: u32,  // [0-100]%
    pub wakeup_threshold: u32, // [0-100]%
    pub stations_count: usize,
    pub active_power: f64, // [W]
    pub sleep_power: f64,  // [W]
    pub wakeup_power: f64, // [W]
    pub wakeup_delay: f64, // [s]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            process_time_max: 0.0,
            process_time_min: 15.0,
            lambda: 1.0,
            lambda_coefs: vec![
                LambdaPoint {
                    coef: 0.5,
                    time: 8.0,
                },
                LambdaPoint {
                    coef: 0.75,
                    time: 6.0,
                },
                LambdaPoint {
                    coef: 1.0,
                    time: 4.0,
                },
                LambdaPoint {
                    coef: 0.75,
                    time: 6.0,
                },
            ],
            resources_count: 10,
            sleep_threshold: 20,
            wakeup_threshold: 80,
            stations_count: 273,
            active_power: 200.0,
            sleep_power: 1.0,
            wakeup_power: 1000.0,
            wakeup_delay: 0.05,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), &str> {
        if self.process_time_min < 0.0 {
            return Err("process_time_min must be greater than 0");
        }
        if self.process_time_max < self.process_time_min {
            return Err("process_time_max must be greater than process_time_min");
        }
        for lp in self.lambda_coefs.iter() {
            if lp.coef < 1.0 {
                return Err("lambda must be greater than 1.0");
            }
            if lp.time < 0.0 {
                return Err("lambda timestamp must be greater than 0");
            }
        }
        if self.sleep_threshold > 100 {
            return Err("sleep_threshold must be from range [0-100]%");
        }
        if self.wakeup_threshold > 100 || self.wakeup_threshold < self.sleep_threshold {
            return Err("wakeup_threshold must be from range [0-100]% and must be greater than sleep_threshold");
        }
        if self.active_power < 0.0 {
            return Err("active_power must be greater than 0");
        }
        if self.sleep_power < 0.0 {
            return Err("sleep_power must be greater than 0");
        }
        if self.wakeup_power < 0.0 {
            return Err("wakeup_power must be greater than 0");
        }
        if self.wakeup_delay < 0.0 {
            return Err("wakeup_delay must be greater than 0");
        }
        Ok(())
    }
}
