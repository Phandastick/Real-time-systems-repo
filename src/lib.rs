pub mod data_structure;
use data_structure::*;

pub mod actuator_lib;
use actuator_lib::*;

pub mod controller_lib;
use controller_lib::*;

use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_micros() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_micros()
}

// src/lib.rs

// pub use actuator::start;
// pub use controller::simulate_controller;

// src/lib.rs
// pub mod controller;

// pub use actuator::simulate_actuator;
// pub use controller::simulate_controller;
