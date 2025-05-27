use std::time::{SystemTime, UNIX_EPOCH};
// src/lib.rs

// pub use actuator::start;
// pub use controller::simulate_controller;

pub fn now_micros() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros()
}
