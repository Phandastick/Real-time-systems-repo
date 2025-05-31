pub fn now_micros() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_micros()
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
//to simulate sensor arm data
pub struct SensorArmData {
    pub object_data: ObjectData,

    pub wrist: WristData,
    pub joints: ShoulderData,
    pub elbow: ElbowData,
    pub arm_velocity: f32,
    //higher speed, more strength
    pub arm_strength: f32, // use speed to calculate force of arm
    pub arm_length: i32,

    pub timestamp: u128,
}
//simulate object data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObjectData {
    pub object_velocity: f32,
    pub object_mass: f32,
    pub object_size: f32,
    pub object_x: f32,
    pub object_y: f32,
    pub object_height: f32,
}

//stored in sensor
//overall data struct in arm
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
            arm_length: 10,
            timestamp: 0,
        }
    }
}
//update object data in sensor arm
impl SensorArmData {
    pub fn update_object_data(&mut self, object_data: ObjectData) {
        self.object_data = object_data;
    }
}
//convert sensor arm data to feedback data
impl SensorArmData {
    pub fn to_feedback(&self, eta: u128) -> FeedbackData {
        FeedbackData {
            wrist: self.wrist.clone(),
            joints: self.joints.clone(),
            elbow: self.elbow.clone(),
            arrived_at_ground: eta,
            timestamp: now_micros(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActuatorInstruction {
    pub x: f32,
    pub y: f32,
    pub strength: f32,
    pub time_to_reach: u64,
    pub timestamp: u128,
}

impl ActuatorInstruction {
    pub fn new(x: f32, y: f32, strength: f32, time_to_reach: u64) -> Self {
        ActuatorInstruction {
            x,
            y,
            strength,
            time_to_reach,
            timestamp: now_micros(),
        }
    }
}

//store feedback data from actuator to sensor
//this is the data that the sensor will use to update its state
//does not have object data, as it is not needed for the feedback
//as long as the arm data has been returned, the object can be deemed caught
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeedbackData {
    pub wrist: WristData,
    pub joints: ShoulderData,
    pub elbow: ElbowData,
    pub arrived_at_ground: u128,

    pub timestamp: u128,
}
//function to get feedback data from sensor arm data
impl SensorArmData {
    pub fn from_feedback(feedback: FeedbackData) -> Self {
        SensorArmData {
            object_data: ObjectData {
                object_velocity: 0.0,
                object_mass: 0.0,
                object_size: 0.0,
                object_height: 0.0,
                object_x: 0.0,
                object_y: 0.0,
            },
            wrist: feedback.wrist,
            joints: feedback.joints,
            elbow: feedback.elbow,
            arm_velocity: 0.0,
            arm_strength: 0.0,
            timestamp: feedback.timestamp,
            arm_length: 10,
        }
    }
}

//considers only the last 5 values
//to smooth out the data
pub const WINDOW_SIZE: usize = 5;
//a simple moving average filter with a fixed window size
#[derive(Debug, Clone)]
pub struct MovingAverage {
    pub buffer: [f32; WINDOW_SIZE],
    pub index: usize,
    pub sum: f32,
    pub count: usize,
}

impl MovingAverage {
    pub fn new() -> Self {
        Self {
            buffer: [0.0; WINDOW_SIZE],
            index: 0,
            sum: 0.0,
            count: 0,
        }
    }
//if the buffer isn't full yet (count < WINDOW_SIZE), it increments count
//else it subtracts the oldest value from the running sum
//it then adds the new value into the buffer at the current index
//updates the sum and advances the circular index
//returns the current average
//O(1) time complexity for each update
    pub fn update(&mut self, val: f32) -> f32 {
        if self.count < WINDOW_SIZE {
            self.count += 1;
        } else {
            self.sum -= self.buffer[self.index];
        }
        self.buffer[self.index] = val;
        self.sum += val;
        self.index = (self.index + 1) % WINDOW_SIZE;
        self.sum / self.count as f32
    }

    pub fn reset(&mut self) {
        self.buffer = [0.0; WINDOW_SIZE];
        self.index = 0;
        self.sum = 0.0;
        self.count = 0;
    }
}

//implement the movingaverage filter for sensorarm data
#[derive(Clone)]
pub struct Filters {
    pub wrist_x_filter: MovingAverage,
    pub wrist_y_filter: MovingAverage,
    pub shoulder_x_filter: MovingAverage,
    pub shoulder_y_filter: MovingAverage,
    pub elbow_x_filter: MovingAverage,
    pub elbow_y_filter: MovingAverage,
    pub arm_velocity_filter: MovingAverage,
    pub object_velocity_filter: MovingAverage,
    pub object_mass_filter: MovingAverage,
    pub object_size_filter: MovingAverage,
    pub object_x_filter: MovingAverage,
    pub object_y_filter: MovingAverage,
    pub object_height_filter: MovingAverage,
}

impl Filters {
    //create a new filters struct with all filters initialized
    pub fn new() -> Self {
        Self {
            wrist_x_filter: MovingAverage::new(),
            wrist_y_filter: MovingAverage::new(),
            shoulder_x_filter: MovingAverage::new(),
            shoulder_y_filter: MovingAverage::new(),
            elbow_x_filter: MovingAverage::new(),
            elbow_y_filter: MovingAverage::new(),
            arm_velocity_filter: MovingAverage::new(),
            object_velocity_filter: MovingAverage::new(),
            object_mass_filter: MovingAverage::new(),
            object_size_filter: MovingAverage::new(),
            object_x_filter: MovingAverage::new(),
            object_y_filter: MovingAverage::new(),
            object_height_filter: MovingAverage::new(),
        }
    }
    //reset all filters
    pub fn reset(&mut self) {
        self.wrist_x_filter.reset();
        self.wrist_y_filter.reset();
        self.shoulder_x_filter.reset();
        self.shoulder_y_filter.reset();
        self.elbow_x_filter.reset();
        self.elbow_y_filter.reset();
        self.arm_velocity_filter.reset();
        self.object_velocity_filter.reset();
        self.object_mass_filter.reset();
        self.object_size_filter.reset();
        self.object_x_filter.reset();
        self.object_y_filter.reset();
        self.object_height_filter.reset();
    }
}
//to track latency in the system of high/normal load
//used to log the time taken for each task
//better to use a struct for clarity
#[derive(Debug)]
pub struct LogEntry {
    pub task: String,
    pub latency: u128, // microseconds
}
