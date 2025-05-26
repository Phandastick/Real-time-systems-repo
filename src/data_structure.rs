use std::time::Duration;

//#region sensor data structures

// FOR SENSOR - updates data in current position and velocity
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SensorData {
    pub position: f64,
    pub velocity: f64,
}

// #region Actuator structures
#[derive(Debug)]
pub struct StampingData {
    // Config
    pub damping_gain: f64, // Damping coefficient (how strongly we respond to predicted vibration)
    pub predictive_horizon: usize, // How many future steps to predict
    pub sampling_interval: Duration, // How often the control loop updates

    // PLatform state
    pub current_position: f64,    // Current actuator/arm position
    pub current_velocity: f64,    // Current speed of motion
    pub vibration_frequency: f64, // Estimated dominant vibration frequency in Hz
    pub vibration_amplitude: f64, // Estimated vibration amplitude in mm or arbitrary units

    // internal buffer
    pub past_disturbances: Vec<f64>, // Stores recent disturbance history for analysis or filtering
}

impl StampingData {
    pub fn new(damping_gain: f64, predictive_horizon: usize, sampling_interval_ms: u64) -> Self {
        StampingData {
            damping_gain,
            predictive_horizon,
            sampling_interval: Duration::from_millis(sampling_interval_ms),
            current_position: 0.0,
            current_velocity: 0.0,
            vibration_frequency: 0.0,
            vibration_amplitude: 0.0,
            past_disturbances: Vec::with_capacity(predictive_horizon),
        }
    }
}

impl StampingData {
    pub fn update_sensor_data(&mut self, position: f64, velocity: f64) {
        self.current_position = position;
        self.current_velocity = velocity;
    }

    pub fn update_vibration_model(&mut self, frequency: f64, amplitude: f64) {
        self.vibration_frequency = frequency;
        self.vibration_amplitude = amplitude;
    }

    pub fn record_disturbance(&mut self, disturbance: f64) {
        if self.past_disturbances.len() >= self.predictive_horizon {
            self.past_disturbances.remove(0); // Keep fixed size buffer
        }
        self.past_disturbances.push(disturbance);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeedbackData {
    pub status: String,
    pub adjustment_value: f64,
    pub timestamp: u128,
}
