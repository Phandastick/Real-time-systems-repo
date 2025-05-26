#[derive(Debug)]
pub struct SensorArmData {
    pub object_angle: f32,
    pub object_mass: f32,

    pub wrist: WristData,
    pub joints: ShoulderData,
    pub elbow: ElbowData,

    pub timestamp: u128,
}

//stored in sensor
#[derive(Debug)]
pub struct WristData {
    pub wrist_x: f32,
    pub wrist_y: f32,
    pub wrist_z: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShoulderData {
    pub shoulder_x: f32,
    pub shoulder_y: f32,
    pub shoulder_z: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ElbowData {
    pub elbow_x: f32,
    pub elbow_y: f32,
    pub elbow_z: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeedbackData {
    pub status: String,
    pub adjustment_value: f64,
    pub timestamp: u128,
}
