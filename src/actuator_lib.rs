use std::thread::sleep;
use std::time::Duration;

// ![#[allow(unused)]
use crate::data_structure::{ElbowData, ObjectData, SensorArmData, ShoulderData, WristData};

/// Computes the joint positions given the input sensor data.
/// Returns the modified `SensorArmData` and the time taken in microseconds.
pub fn compute_arm_movement(mut data: SensorArmData) -> SensorArmData {
    let target_x = data.object_data.object_x;
    let target_y = data.object_data.object_y;

    let l1 = 10.0;
    let l2 = 10.0;

    let dist = (target_x.powi(2) + target_y.powi(2)).sqrt();

    let (clamped_x, clamped_y) = if dist > l1 + l2 {
        let scale = (l1 + l2) / dist;
        (target_x * scale, target_y * scale)
    } else {
        (target_x, target_y)
    };

    let cos_theta2 =
        ((clamped_x.powi(2) + clamped_y.powi(2)) - l1.powi(2) - l2.powi(2)) / (2.0 * l1 * l2);
    let cos_theta2 = cos_theta2.clamp(-1.0, 1.0);
    let theta2 = cos_theta2.acos();

    let k1 = l1 + l2 * theta2.cos();
    let k2 = l2 * theta2.sin();
    let theta1 = clamped_y.atan2(clamped_x) - k2.atan2(k1);

    let shoulder_x = 0.0;
    let shoulder_y = 0.0;

    let elbow_x = shoulder_x + l1 * theta1.cos();
    let elbow_y = shoulder_y + l1 * theta1.sin();

    let wrist_x = elbow_x + l2 * (theta1 + theta2).cos();
    let wrist_y = elbow_y + l2 * (theta1 + theta2).sin();

    data.joints.shoulder_x = shoulder_x;
    data.joints.shoulder_y = shoulder_y;

    data.elbow.elbow_x = elbow_x;
    data.elbow.elbow_y = elbow_y;

    data.wrist.wrist_x = wrist_x;
    data.wrist.wrist_y = wrist_y;

    shoulder_movement(data.joints.clone());
    elbow_movement(data.elbow.clone());

    data
}

//SPAWN SHOULDER JOINT, ELBOW JOINT THREADS and CHANNEL
// shoudler thread
fn shoulder_movement(data: ShoulderData) {
    println!(
        "[SHOULDER] Moving to position x:{:?} y:{:?}",
        data.shoulder_x, data.shoulder_y
    );

    // sleep(Duration::from_millis(1));
}

//elbow thread
fn elbow_movement(data: ElbowData) {
    println!(
        "[ELBOW] Moving to position x:{:?} y:{:?}",
        data.elbow_x, data.elbow_y
    );
    // sleep(Duration::from_millis(1));
}
