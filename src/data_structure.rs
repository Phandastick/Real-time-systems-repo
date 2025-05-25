#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SensorArmData {
    pub force_data: f32,
    pub wrist: WristData,
    pub joints: ShoulderData,
    pub elbow: ElbowData,
    pub timestamp: u128,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    pub actuator_id: u32,      // 1, for wrist, 2 for shoulder, 3 for elbow
    pub status: String,        // e.g., "ok", "error", "calibrating"
    pub adjustment_value: f64, // Suggests how the sensor should react (e.g., calibrate)
    pub timestamp: u128,
}
