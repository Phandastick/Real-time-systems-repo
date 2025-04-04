use std::{thread::sleep, time::Duration};

pub fn sensor_start() {
    print!("Sensor module started\n");

    for _ in 1..10 {
        sleep(Duration::from_millis(100));
        println!("Sensor says hi");
    }
}
