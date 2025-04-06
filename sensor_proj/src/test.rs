use std::{thread::sleep, time::Duration};

use crate::data_structure::{ElbowData, SensorArmData, ShoulderData, WristData};

pub fn start() {
    // Create shared memory segment with 1 KB
    let mut shmem = SharedMem::create("sensor_shmem", 1024).unwrap();

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
