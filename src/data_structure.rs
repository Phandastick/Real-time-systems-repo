use std::os::raw::c_float;
pub struct WristData {
    wrist_x: c_float,
    wrist_y: c_float,
    wrist_z: c_float,
}

pub struct ShoulderData {
    shoulder_x: c_float,
    shoulder_y: c_float,
    shoulder_z: c_float,
}

pub struct ElbowData {
    elbow_x: c_float,
    elbow_y: c_float,
    elbow_z: c_float,
}

pub struct SensorArmData {
    force_data: c_float,
    wrist: WristData,
    joints: ShoulderData,
    elbow: ElbowData,
}