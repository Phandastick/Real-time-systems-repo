#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SensorArmData {
    pub object_data: ObjectData,

    pub wrist: WristData,
    pub shoulder: ShoulderData, // controls x
    pub elbow: ElbowData,       // controls y
    pub arm_velocity: f32,
    //higher speed, more strength
    pub arm_strength: f32, // use speed to calculate force of arm

    pub timestamp: u128,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObjectData {
    pub object_velocity: f32,
    pub object_mass: f32,
    pub object_size: f32,
    pub object_distance: f32, //height from robotic arm
}

//stored in sensor
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WristData {
    pub wrist_x: f32,
    pub wrist_y: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShoulderData {
    pub shoulder_x: f32,
    pub shoulder_y: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ElbowData {
    pub elbow_x: f32,
    pub elbow_y: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeedbackData {
    pub wrist: WristData,
    pub shoulder: ShoulderData,
    pub elbow: ElbowData,
}
