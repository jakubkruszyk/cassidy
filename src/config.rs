use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to config file. This option cannot be used with any other config switch.
    #[arg(long, value_name = "path")]
    pub with_config: Option<PathBuf>,
    /// Seed for random number generator
    #[arg(long, value_name = "u64")]
    pub seed: Option<u64>,
    /// Generate log file
    #[arg(long)]
    pub log: bool,
    /// Path where log file will be saved to
    #[arg(long, value_name = "path")]
    pub log_path: Option<PathBuf>,
    /// Time in hours, simulation will be run for. Maximum precision is 1ms
    #[arg(long, value_name = "time")]
    pub duration: f64,
    /// Simulation iterations count
    #[arg(long, value_name = "u32")]
    pub iterations: u32,
    /// Enable sleep state logic
    #[arg(long)]
    pub enable_sleep: bool,
    /// Save default config
    #[arg(long, value_name = "path")]
    pub save_default_config: Option<PathBuf>,
    /// Show partial results from all iterations
    #[arg(long)]
    pub show_partial_results: bool,
    /// Log simulation process in binary format
    #[arg(long)]
    pub log_wave: bool,
    /// Binary log sampling divider
    #[arg(long, value_name = "u32", default_value_t = 1)]
    pub samples: usize,
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

    pub fn validate(self) -> Result<Cli, String> {
        if self.duration < 0.0 {
            return Err("Duration must be greater than 0".to_string());
        }
        if let Some(path) = &self.with_config {
            if !path.exists() {
                return Err(format!(
                    "Given path to config file: '{}' does not exists",
                    &path.display()
                ));
            }
        }
        if self.iterations == 0 {
            return Err("Iterations must be greater than 0".to_owned());
        }
        Ok(self)
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
    pub process_time_max: u64, // [ms]
    pub process_time_min: u64, // [ms]
    pub lambda: f64,           // [users per second]
    pub lambda_coefs: Vec<LambdaPoint>,
    pub resources_count: usize,
    pub sleep_threshold: u32,  // [0-100]%
    pub wakeup_threshold: u32, // [0-100]%
    pub stations_count: usize,
    pub active_power: f64, // [W]
    pub sleep_power: f64,  // [W]
    pub wakeup_power: f64, // [W]
    pub wakeup_delay: u64, // [ms]
    pub log_buffer: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            process_time_max: 30000,
            process_time_min: 1000,
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
            resources_count: 273,
            sleep_threshold: 20,
            wakeup_threshold: 80,
            stations_count: 10,
            active_power: 200.0,
            sleep_power: 1.0,
            wakeup_power: 1000.0,
            wakeup_delay: 50,
            log_buffer: 1000,
        }
    }
}

impl Config {
    pub fn validate(self) -> Result<Config, String> {
        if self.lambda < 0.0 {
            return Err("lambda must be greater than 0".to_owned());
        }
        if self.lambda_coefs.len() == 0 {
            return Err("lambda_coefs list must not be empty.".to_owned());
        }
        for lp in self.lambda_coefs.iter() {
            if lp.coef < 0.0 {
                return Err("lambda coefficient must be greater than 0".to_owned());
            }
            if lp.time < 0.0 {
                return Err("lambda timestamp must be greater than 0".to_owned());
            }
        }
        if self.sleep_threshold > 100 {
            return Err("sleep_threshold must be from range [0-100]%".to_owned());
        }
        if self.wakeup_threshold > 100 || self.wakeup_threshold < self.sleep_threshold {
            return Err("wakeup_threshold must be from range [0-100]% and must be greater than sleep_threshold".to_owned());
        }
        if self.sleep_threshold >= self.wakeup_threshold / 2 {
            println!("Warning] sleep_threshold: {} is greater than wakeup_threshold / 2: {}. This can cause oscillations in stations state", self.sleep_threshold, self.wakeup_threshold)
        }
        if self.active_power < 0.0 {
            return Err("active_power must be greater than 0".to_owned());
        }
        if self.sleep_power < 0.0 {
            return Err("sleep_power must be greater than 0".to_owned());
        }
        if self.wakeup_power < 0.0 {
            return Err("wakeup_power must be greater than 0".to_owned());
        }
        Ok(self)
    }

    pub fn save_deafult(path: PathBuf) -> std::io::Result<()> {
        let cfg = Config::default();
        let cfg_str =
            toml::to_string(&cfg).expect("Internal error: Couldn't parse default config to toml.");
        let mut file = File::create(&path)?;
        file.write_all(cfg_str.as_bytes())?;
        Ok(())
    }
}
