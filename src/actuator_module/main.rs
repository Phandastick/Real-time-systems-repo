#![allow(dead_code)]
use std::os::raw::c_float;

struct WristData {
    wrist_x: c_float,
    wrist_y: c_float,
    wrist_z: c_float,
}

struct JointData {
    shoulder_x: c_float,
    shoulder_y: c_float,
    shoulder_z: c_float,
}

struct ElbowData {
    elbow_x: c_float,
    elbow_y: c_float,
    elbow_z: c_float,
}

struct SensorArmData {
    force_data: c_float,
    wrist: WristData,
    joints: JointData,
    elbow: ElbowData,
}

pub fn actuator_start() {
    println!("hello world")
}
