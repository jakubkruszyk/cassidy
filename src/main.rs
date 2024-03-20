use crate::{config::Config, sim_container::SimContainer};

mod basestation;
mod config;
mod logger;
mod sim_container;
mod user;

fn main() {
    let mut scene = match SimContainer::new() {
        Ok(s) => s,
        Err(e) => panic!("{}", e),
    };
    if let Some(path) = &scene.cli.save_default_config {
        match Config::save_deafult(path.clone()) {
            Err(e) => panic!("{}", e),
            Ok(_) => (),
        };
    }
    let res = scene.run();
    println!("\n=================== Average simulation results ===================");
    println!("{}", res.get_report());
}
