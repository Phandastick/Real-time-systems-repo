use lapin::{options::*, BasicProperties, Connection, ConnectionProperties};
use rand::random;
use Real_time_systems_repo::data_structure::*;
use serde_json;
use std::time::{Instant, Duration};

const WINDOW_SIZE: usize = 5;

#[derive(Debug)]
struct MovingAverage {
    buffer: [f32; WINDOW_SIZE],
    index: usize,
    sum: f32,
    count: usize,
}

impl MovingAverage {
    fn new() -> Self {
        Self {
            buffer: [0.0; WINDOW_SIZE],
            index: 0,
            sum: 0.0,
            count: 0,
        }
    }

    fn update(&mut self, val: f32) -> f32 {
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
}

struct Filters {
    force_filter: MovingAverage,
    wrist_x_filter: MovingAverage,
    wrist_y_filter: MovingAverage,
    wrist_z_filter: MovingAverage,
    shoulder_x_filter: MovingAverage,
    shoulder_y_filter: MovingAverage,
    shoulder_z_filter: MovingAverage,
    elbow_x_filter: MovingAverage,
    elbow_y_filter: MovingAverage,
    elbow_z_filter: MovingAverage,
}

impl Filters {
    fn new() -> Self {
        Self {
            force_filter: MovingAverage::new(),
            wrist_x_filter: MovingAverage::new(),
            wrist_y_filter: MovingAverage::new(),
            wrist_z_filter: MovingAverage::new(),
            shoulder_x_filter: MovingAverage::new(),
            shoulder_y_filter: MovingAverage::new(),
            shoulder_z_filter: MovingAverage::new(),
            elbow_x_filter: MovingAverage::new(),
            elbow_y_filter: MovingAverage::new(),
            elbow_z_filter: MovingAverage::new(),
        }
    }
}

fn detect_anomaly_force(value: f32) -> bool {
    value > 15.0
}

fn detect_anomaly_joint(value: f32) -> bool {
    value < 0.0 || value > 1.0
}

fn generate_sensor_data(cycle: u64) -> SensorArmData {
    let mut force = random::<f32>() * 10.0;
    if cycle % 50 == 0 {
        force += 20.0; // inject anomaly every 50 cycles
    }
    SensorArmData {
        force_data: force,
        wrist: WristData {
            wrist_x: random::<f32>(),
            wrist_y: random::<f32>(),
            wrist_z: random::<f32>(),
        },
        joints: ShoulderData {
            shoulder_x: random::<f32>(),
            shoulder_y: random::<f32>(),
            shoulder_z: random::<f32>(),
        },
        elbow: ElbowData {
            elbow_x: random::<f32>(),
            elbow_y: random::<f32>(),
            elbow_z: random::<f32>(),
        },
    }
}

fn process_sensor_data(raw: &SensorArmData, filters: &mut Filters) -> (SensorArmData, bool) {
    let mut filtered = raw.clone();

    filtered.force_data = filters.force_filter.update(raw.force_data);
    filtered.wrist.wrist_x = filters.wrist_x_filter.update(raw.wrist.wrist_x);
    filtered.wrist.wrist_y = filters.wrist_y_filter.update(raw.wrist.wrist_y);
    filtered.wrist.wrist_z = filters.wrist_z_filter.update(raw.wrist.wrist_z);

    filtered.joints.shoulder_x = filters.shoulder_x_filter.update(raw.joints.shoulder_x);
    filtered.joints.shoulder_y = filters.shoulder_y_filter.update(raw.joints.shoulder_y);
    filtered.joints.shoulder_z = filters.shoulder_z_filter.update(raw.joints.shoulder_z);

    filtered.elbow.elbow_x = filters.elbow_x_filter.update(raw.elbow.elbow_x);
    filtered.elbow.elbow_y = filters.elbow_y_filter.update(raw.elbow.elbow_y);
    filtered.elbow.elbow_z = filters.elbow_z_filter.update(raw.elbow.elbow_z);

    let anomaly = detect_anomaly_force(filtered.force_data)
        || detect_anomaly_joint(filtered.wrist.wrist_x)
        || detect_anomaly_joint(filtered.wrist.wrist_y)
        || detect_anomaly_joint(filtered.wrist.wrist_z)
        || detect_anomaly_joint(filtered.joints.shoulder_x)
        || detect_anomaly_joint(filtered.joints.shoulder_y)
        || detect_anomaly_joint(filtered.joints.shoulder_z)
        || detect_anomaly_joint(filtered.elbow.elbow_x)
        || detect_anomaly_joint(filtered.elbow.elbow_y)
        || detect_anomaly_joint(filtered.elbow.elbow_z);

    (filtered, anomaly)
}

#[tokio::main]
async fn main() {
    // Connect to RabbitMQ server
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
        .await
        .expect("Connection error");

    let channel = conn.create_channel().await.expect("Channel creation error");

    // Declare the queue (make sure it matches the consumer queue name)
    channel
        .queue_declare(
            "sensor_data",
            QueueDeclareOptions::default(),
            Default::default(),
        )
        .await
        .expect("Queue declaration error");

    let mut filters = Filters::new();

    let mut cycle: u64 = 0;

    loop {
        cycle += 1;
        let start = Instant::now();

        let raw_data = generate_sensor_data(cycle);
        let (processed_data, anomaly) = process_sensor_data(&raw_data, &mut filters);

        println!(
            "cycle: {:03} force: {:.2} anomaly: {}",
            cycle, processed_data.force_data, anomaly
        );

        // Serialize processed data to JSON
        let payload = serde_json::to_vec(&processed_data).expect("Failed to serialize");

        // Publish message to RabbitMQ queue "sensor_data"
        channel
            .basic_publish(
                "",
                "sensor_data",
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await
            .expect("Failed to publish")
            .await
            .expect("Publisher confirm failed");

        // Keep the loop at ~5ms interval
        let elapsed = start.elapsed();
        if elapsed < Duration::from_millis(5) {
            tokio::time::sleep(Duration::from_millis(5) - elapsed).await;
        }
    }
}
