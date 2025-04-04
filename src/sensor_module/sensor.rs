use crate::data_structure::{ElbowData, SensorArmData, ShoulderData, WristData};

pub fn sensor() {
    //Arm will start out at a bent 90 degree angle
    let mut arm_data = SensorArmData {
        force_data: 0.0,
        wrist: WristData {
            wrist_x: 1.0,
            wrist_y: 0.0,
            wrist_z: 0.0,
        },
        joints: ShoulderData {
            shoulder_x: 0.0,
            shoulder_y: 1.0,
            shoulder_z: 0.0,
        },
        elbow: ElbowData {
            elbow_x: 1.0,
            elbow_y: 0.0,
            elbow_z: 0.0,
        },
    };
}
