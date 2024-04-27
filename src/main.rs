use crate::{
    config::{Cli, Config},
    sim_container::SimContainer,
};
use clap::Parser;
use rayon::prelude::*;
use sim_container::SimResults;
use std::{io::Write, iter::zip};

mod basestation;
mod config;
mod logger;
mod sim_container;
mod user;

fn main() {
    let cli = match Cli::parse().validate() {
        Ok(c) => c,
        Err(e) => panic!("{}", e),
    };
    if let Some(path) = &cli.save_default_config {
        match Config::save_deafult(path.clone()) {
            Err(e) => panic!("{}", e),
            Ok(_) => (),
        };
    }
    let walk_cfg = match cli.create_walk_over_config() {
        Ok(c) => c,
        Err(e) => panic!("{}", e),
    };

    let report = match walk_cfg {
        Some(walk_cfg) => {
            let mut points: Vec<f64> = Vec::new();
            let mut point = walk_cfg.start;
            while point <= walk_cfg.end {
                points.push(point);
                point += walk_cfg.step;
            }
            let results: Vec<SimResults> = points
                .par_iter()
                .enumerate()
                .map(|(idx, p)| {
                    let mut scene = match SimContainer::new() {
                        Ok(s) => s,
                        Err(e) => panic!("{}", e),
                    };
                    scene.update_param(&walk_cfg.var, *p);
                    return scene.run(idx);
                })
                .collect();

            let mut report = format!("{},{}\n", walk_cfg.var, results[0].get_csv_header());
            for (param, res) in zip(points.iter(), results.iter()) {
                report += &format!("{},", param);
                report += &res.get_csv();
                report += "\n";
            }
            report
        }
        None => {
            let scene = match SimContainer::new() {
                Ok(s) => s,
                Err(e) => panic!("{}", e),
            };
            let res = scene.run(0);
            let report = res.get_report();
            println!("\n=================== Average simulation results ===================");
            println!("{}", report);
            report
        }
    };
    let file = std::fs::File::create("sim_report");
    match file {
        Ok(mut f) => f.write_all(report.as_bytes()).unwrap(),
        Err(e) => println!("{}", e),
    };
    println!("Simulation finished. Results saved in sim_report file.");
}
