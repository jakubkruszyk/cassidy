use sim_container::SimContainer;

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
    scene.simulate();
}
