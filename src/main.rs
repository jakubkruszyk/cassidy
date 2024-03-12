mod basestation;
mod config;
mod logger;
mod sim_container;
mod user;

use sim_container::SimContainer;

fn main() {
    let scene = SimContainer::new();
}
