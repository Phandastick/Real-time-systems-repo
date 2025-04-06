#![allow(dead_code)]

mod data_structure;
mod test;

use std::{thread::sleep, time::Duration};

fn main() {
    println!("Actuator starting...");

    test::start();
}
