use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use rand::random;
use serde_json;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use Real_time_systems_repo::data_structure::*;
use futures_util::stream::StreamExt;
use scheduled_thread_pool::ScheduledThreadPool;
use tokio::sync::mpsc;
use tokio::sync::Notify;

const WINDOW_SIZE: usize = 5;

#[derive(Debug, Clone)]
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

fn now_micros() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros()
}

#[derive(Clone)]
struct Filters {
    wrist_x_filter: MovingAverage,
    wrist_y_filter: MovingAverage,
    shoulder_x_filter: MovingAverage,
    shoulder_y_filter: MovingAverage,
    elbow_x_filter: MovingAverage,
    elbow_y_filter: MovingAverage,
    arm_velocity_filter: MovingAverage,
    object_distance_filter: MovingAverage,
}

impl Filters {
    fn new() -> Self {
        Self {
            wrist_x_filter: MovingAverage::new(),
            wrist_y_filter: MovingAverage::new(),
            shoulder_x_filter: MovingAverage::new(),
            shoulder_y_filter: MovingAverage::new(),
            elbow_x_filter: MovingAverage::new(),
            elbow_y_filter: MovingAverage::new(),
            arm_velocity_filter: MovingAverage::new(),
            object_distance_filter: MovingAverage::new(),
        }
    }
}

fn detect_anomaly(value: f32, lower: f32, upper: f32) -> bool {
    value < lower || value > upper
}

fn generate_sensor_data(cycle: u64) -> SensorArmData {
    let object_data = ObjectData {
        object_velocity: random::<f32>() * 5.0,
        object_mass: 1.0 + random::<f32>() * 4.0,
        object_size: random::<f32>() * 2.0,
        object_distance: 0.5 + random::<f32>(),
    };

    let mut sensor_data = SensorArmData::new(object_data.clone());
    sensor_data.update_object_data(object_data);

    sensor_data.wrist.wrist_x = random::<f32>();
    sensor_data.wrist.wrist_y = random::<f32>();

    sensor_data.joints.shoulder_x = random::<f32>();
    sensor_data.joints.shoulder_y = random::<f32>();

    sensor_data.elbow.elbow_x = random::<f32>();
    sensor_data.elbow.elbow_y = random::<f32>();

    sensor_data.arm_velocity = random::<f32>() * 10.0;
    sensor_data.arm_strength = sensor_data.arm_velocity * sensor_data.object_data.object_mass;
    sensor_data.timestamp = now_micros();

    sensor_data
}

fn process_sensor_data(raw: SensorArmData, filters: &mut Filters) -> (SensorArmData, bool) {
    let mut filtered = raw.clone();

    filtered.wrist.wrist_x = filters.wrist_x_filter.update(raw.wrist.wrist_x);
    filtered.wrist.wrist_y = filters.wrist_y_filter.update(raw.wrist.wrist_y);
    filtered.joints.shoulder_x = filters.shoulder_x_filter.update(raw.joints.shoulder_x);
    filtered.joints.shoulder_y = filters.shoulder_y_filter.update(raw.joints.shoulder_y);
    filtered.elbow.elbow_x = filters.elbow_x_filter.update(raw.elbow.elbow_x);
    filtered.elbow.elbow_y = filters.elbow_y_filter.update(raw.elbow.elbow_y);
    filtered.arm_velocity = filters.arm_velocity_filter.update(raw.arm_velocity);
    filtered.object_data.object_distance = filters.object_distance_filter.update(raw.object_data.object_distance);
    filtered.arm_strength = filtered.arm_velocity * raw.object_data.object_mass;

    let anomaly = detect_anomaly(filtered.arm_strength, 0.0, 100.0)
        || detect_anomaly(filtered.wrist.wrist_x, 0.0, 1.0)
        || detect_anomaly(filtered.wrist.wrist_y, 0.0, 1.0)
        || detect_anomaly(filtered.joints.shoulder_x, 0.0, 1.0)
        || detect_anomaly(filtered.joints.shoulder_y, 0.0, 1.0)
        || detect_anomaly(filtered.elbow.elbow_x, 0.0, 1.0)
        || detect_anomaly(filtered.elbow.elbow_y, 0.0, 1.0);

    (filtered, anomaly)
}

async fn consume_feedback(shutdown: Arc<Notify>) {
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
        .await.expect("Connection error");

    let channel = conn.create_channel().await.expect("Channel creation error");

    channel.queue_declare(
        "feedback_to_sensor",
        QueueDeclareOptions::default(),
        FieldTable::default(),
    ).await.expect("Queue declaration error");

    let mut consumer = channel.basic_consume(
        "feedback_to_sensor",
        "feedback_consumer",
        BasicConsumeOptions::default(),
        FieldTable::default(),
    ).await.expect("Basic consume error");

    println!("> Feedback consumer ready...");

    loop {
        tokio::select! {
            maybe_delivery = consumer.next() => {
                if let Some(Ok(delivery)) = maybe_delivery {
                    let payload = &delivery.data;
                    let feedback: FeedbackData = match serde_json::from_slice(payload) {
                        Ok(fb) => fb,
                        Err(e) => {
                            eprintln!("Failed to deserialize feedback: {:?}", e);
                            delivery.nack(Default::default()).await.expect("Failed to nack");
                            continue;
                        }
                    };
                    println!("Received feedback: {:?}", feedback);
                    delivery.ack(Default::default()).await.expect("Failed to ack");
                } else {
                    break;
                }
            }
            _ = shutdown.notified() => {
                println!("Feedback consumer received shutdown signal.");
                break;
            }
        }
    }

    println!("Feedback consumer exiting cleanly.");
}

#[tokio::main]
async fn main() {
    let pool = ScheduledThreadPool::new(4);
    let cycle = Arc::new(Mutex::new(1u64));
    let max_cycles = 1_000u64;
    let shared_filters = Arc::new(Mutex::new(Filters::new()));
    let shared_filters_clone = Arc::clone(&shared_filters);
    let (tx_processed, mut rx_processed) = mpsc::channel::<SensorArmData>(100);
    let shutdown_notify = Arc::new(Notify::new());
    let shutdown_notify_producer = Arc::clone(&shutdown_notify);
    let feedback_shutdown = Arc::new(Notify::new());
    let feedback_shutdown_consumer = Arc::clone(&feedback_shutdown);

    let publisher_handle = tokio::spawn(async move {
        let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
            .await.expect("Connection error");
        let channel = conn.create_channel().await.expect("Channel creation error");

        channel.queue_declare(
            "sensor_data",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        ).await.expect("Queue declaration error");

        while let Some(processed_data) = rx_processed.recv().await {
            let payload = serde_json::to_vec(&processed_data).expect("Serialization failed");

            channel.basic_publish(
                "",
                "sensor_data",
                BasicPublishOptions::default(),
                &payload,
                Default::default(),
            ).await.expect("Publish failed").await.expect("Confirmation failed");
        }
        println!("Publisher exiting cleanly.");
    });

    let feedback_handle = tokio::spawn(async move {
        consume_feedback(feedback_shutdown_consumer).await;
    });

    let tx_blocking = tx_processed.clone();
    let cycle_clone = Arc::clone(&cycle);

    pool.execute_at_fixed_rate(Duration::from_millis(0), Duration::from_millis(10), move || {
        let mut c = cycle_clone.lock().unwrap();
        if *c > max_cycles {
            shutdown_notify_producer.notify_waiters();
            return;
        }

        let current_cycle = *c;
        *c += 1;

        let data = generate_sensor_data(current_cycle);
        let mut filters = shared_filters_clone.lock().unwrap();
        let (processed, anomaly) = process_sensor_data(data, &mut filters);

        println!(
            "cycle {:03}, arm_strength: {:.2}, anomaly: {}",
            current_cycle, processed.arm_strength, anomaly
        );

        if let Err(e) = tx_blocking.try_send(processed) {
            eprintln!("Failed to send processed data: {}", e);
        }
    });

    shutdown_notify.notified().await;
    println!("All cycles processed. Cleaning up...");
    drop(pool);
    drop(tx_processed);
    feedback_shutdown.notify_waiters();

    publisher_handle.await.expect("Publisher panicked");
    feedback_handle.await.expect("Feedback panicked");
    println!("Shutdown complete. Exiting.");
}
