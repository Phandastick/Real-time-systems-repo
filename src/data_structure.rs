use crate::now_micros;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SensorArmData {
    pub object_data: ObjectData,

    pub wrist: WristData,
    pub joints: ShoulderData,
    pub elbow: ElbowData,
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
    pub object_x: f32,        // x position of the object
    pub object_y: f32,        // y position of the object
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

impl SensorArmData {
    pub fn new(object_data: ObjectData) -> Self {
        let joints = ShoulderData {
            shoulder_x: 0.0,
            shoulder_y: 0.0,
        };
        let elbow = ElbowData {
            elbow_x: 0.0,
            elbow_y: 3.0,
        };
        let wrist = WristData {
            wrist_x: joints.shoulder_x,
            wrist_y: elbow.elbow_y,
        };

        let arm_velocity = 1.0;
        let arm_strength = arm_velocity * 10.0;

        SensorArmData {
            object_data,
            wrist,
            joints,
            elbow,
            arm_velocity,
            arm_strength,
            timestamp: 0,
        }
    }
}

impl SensorArmData {
    pub fn update_object_data(&mut self, object_data: ObjectData) {
        self.object_data = object_data;
    }
}
impl SensorArmData {
    pub fn to_feedback(&self) -> FeedbackData {
        FeedbackData {
            wrist: self.wrist.clone(),
            joints: self.joints.clone(),
            elbow: self.elbow.clone(),
            timestamp: now_micros(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeedbackData {
    pub wrist: WristData,
    pub joints: ShoulderData,
    pub elbow: ElbowData,
    pub timestamp: u128,
}

impl SensorArmData {
    pub fn from_feedback(feedback: FeedbackData) -> Self {
        SensorArmData {
            object_data: ObjectData {
                object_velocity: 0.0,
                object_mass: 0.0,
                object_size: 0.0,
                object_distance: 0.0,
                object_x: 0.0,
                object_y: 0.0,
            },
            wrist: feedback.wrist,
            joints: feedback.joints,
            elbow: feedback.elbow,
            arm_velocity: 0.0,
            arm_strength: 0.0,
            timestamp: feedback.timestamp,
        }
    }
}
