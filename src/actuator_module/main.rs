#![allow(dead_code)]

use std::{thread::sleep, time::Duration};

pub async fn actuator_start() {
    println!("Actuator starting...");
    for _ in 1..10 {
        sleep(Duration::from_millis(100));
        println!("Actuator says hi");
    }
}
