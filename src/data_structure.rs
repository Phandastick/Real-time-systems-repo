use std::time::Duration;

//#region sensor data structures
// FOR SENSOR - updates data in current position and velocity
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SensorData {
    pub offset_x: f64, // Current position offset in X
    pub offset_y: f64, // Current position offset in Y
}

// #region Actuator structures
#[derive(Debug)]
pub struct StampingData {
    // Config
    pub damping_gain: f64, // Damping coefficient (how strongly we respond to predicted vibration)

    // Platform state
    pub offset_x: f64, // Current actuator/arm position offset
    pub offset_y: f64, // Current actuator/arm position offset
}

impl StampingData {
    pub fn new(damping_gain: f64) -> Self {
        StampingData {
            damping_gain,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

impl StampingData {
    pub fn update_sensor_data(&mut self, offset_x: f64, offset_y: f64) {
        self.offset_x = offset_x;
        self.offset_y = offset_y;
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeedbackData {
    pub remaining_offset_x: f64, // Remaining offset in X after processing
    pub remaining_offset_y: f64, // Remaining offset in Y after processing
}
