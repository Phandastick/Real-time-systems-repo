#![allow(dead_code)]
use std::os::raw::c_float;
pub struct WristData {
    pub(crate) wrist_x: c_float,
    pub(crate) wrist_y: c_float,
    pub(crate) wrist_z: c_float,
}

pub struct ShoulderData {
    pub(crate) shoulder_x: c_float,
    pub(crate) shoulder_y: c_float,
    pub(crate) shoulder_z: c_float,
}

pub struct ElbowData {
    pub(crate) elbow_x: c_float,
    pub(crate) elbow_y: c_float,
    pub(crate) elbow_z: c_float,
}

pub struct SensorArmData {
    pub(crate) force_data: c_float,
    pub(crate) wrist: WristData,
    pub(crate) joints: ShoulderData,
    pub(crate) elbow: ElbowData,
}
