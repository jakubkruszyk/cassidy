use crate::sim_container::SimContainer;

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
    let res = scene.run();
    println!("=================== Average simulation results ===================");
    println!("{}", res.get_report());
}
