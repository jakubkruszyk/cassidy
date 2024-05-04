use crate::basestation::BaseStationResult;
use crate::config::Config;
use core::iter::zip;

#[derive(Debug)]
pub struct SimResults {
    pub average_usage: f64,
    pub average_power: f64,
    pub average_drop_rate: f64,
    pub total_users: usize,
    pub dropped_users: usize,
    pub stations: Vec<BaseStationResult>,
}

impl SimResults {
    pub fn new_zero(cfg: &Config) -> SimResults {
        let mut res = SimResults {
            average_usage: 0.0,
            average_power: 0.0,
            average_drop_rate: 0.0,
            total_users: 0,
            dropped_users: 0,
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
        self.dropped_users += x.dropped_users;
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
        self.dropped_users = (self.dropped_users as f64 / x) as usize;
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
            - dropped users: {} \n\
            - average resource usage: {:.2} %\n\
            - average power consumption: {:.2} W\n\
            - average user drop rate: {:.2} %\n\
            \n\
            Stations results:\n\
            id  | average power [W] | average usage [%] | average sleep time [%]\n\
            ----+-------------------+-------------------+-----------------------\n",
            self.total_users,
            self.dropped_users,
            self.average_usage,
            self.average_power,
            self.average_drop_rate
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
            "processed_users,dropped_users,average_resource_usage,average_power_consumption,average_user_drop_rate"
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
            "{},{},{},{},{}",
            self.total_users,
            self.dropped_users,
            self.average_usage,
            self.average_power,
            self.average_drop_rate
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
