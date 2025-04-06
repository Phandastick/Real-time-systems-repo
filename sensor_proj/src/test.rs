#[allow(unused_imports)]
use shared_memory::{Shmem, ShmemConf, ShmemError};
use std::{thread::sleep, time::Duration};

use crate::data_structure::{ElbowData, SensorArmData, ShoulderData, WristData};

pub fn start() {
    // Create shared memory segment with 1 KB
    // let mut shmem = ::create("sensor_shmem", 1024).unwrap();

    let shmem = ShmemConf::new();
    shmem.size(1024);
    shmem.flink("sensor_shmem");

    //init shemem
    let shmem = shmem.create();

    let shmem = match ShmemConf::new().size(1024).flink("sensor_shmem").create() {
        Ok(m) => m,
        Err(ShmemError::LinkExists) => ShmemConf::new().flink("sensor_shmem").open().unwrap(),
        Err(e) => {
            eprintln!("Unable to create or open shmem flink sensor_shmem : {e}");
            return;
        }
    };

    let raw_ptr = shmem.as_ptr();

    // Serialize struct
    let wrist = WristData {
        wrist_x: 0.0,
        wrist_y: 0.0,
        wrist_z: 0.0,
    };

    let shoulder = ShoulderData {
        shoulder_x: 0.0,
        shoulder_y: 0.0,
        shoulder_z: 0.0,
    };

    let elbow = ElbowData {
        elbow_x: 0.0,
        elbow_y: 0.0,
        elbow_z: 0.0,
    };

    let arm = SensorArmData {
        force_data: 0.0,
        wrist,
        joints: shoulder,
        elbow,
    };

    // Example use
    println!("Wrist x: {}", arm.wrist.wrist_x);
    let encoded = serde::serde_derive(&data).unwrap();

    // Write to shared memory
    shmem.as_slice_mut()[..encoded.len()].copy_from_slice(&encoded);

    println!("Sensor: Data written to shared memory.");
    sleep(Duration::from_secs(10)); // So it stays open for a while
}
