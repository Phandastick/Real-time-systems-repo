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

fn now_micros() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros()
}

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

#[derive(Clone)]
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
        force += 20.0;
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
        timestamp: now_micros(),
    }
}

fn process_sensor_data(raw: SensorArmData, filters: &mut Filters) -> (SensorArmData, bool) {
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
                    break; // consumer ended
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
    //notifiers for shutdown 
    let shutdown_notify = Arc::new(Notify::new());
    let shutdown_notify_producer = Arc::clone(&shutdown_notify);
    let feedback_shutdown = Arc::new(Notify::new());
    let feedback_shutdown_consumer = Arc::clone(&feedback_shutdown);

    //publisher
    let publisher_handle = tokio::spawn(async move {
        let conn = lapin::Connection::connect("amqp://127.0.0.1:5672/%2f", lapin::ConnectionProperties::default())
            .await.expect("Connection error");
        let channel = conn.create_channel().await.expect("Channel creation error");

        channel.queue_declare(
            "sensor_data",
            lapin::options::QueueDeclareOptions::default(),
            lapin::types::FieldTable::default(),
        )
        .await.expect("Queue declaration error");

        channel.basic_qos(1, lapin::options::BasicQosOptions::default())
            .await.expect("QoS error");

        while let Some(processed_data) = rx_processed.recv().await {
            let payload = serde_json::to_vec(&processed_data).expect("Serialization failed");

            channel.basic_publish(
                "",
                "sensor_data",
                lapin::options::BasicPublishOptions::default(),
                &payload,
                lapin::BasicProperties::default(),
            )
            .await.expect("Publish failed")
            .await.expect("Confirmation failed");
        }

        println!("Publisher task exiting cleanly.");
    });

    //feedback
    let feedback_handle = tokio::spawn(async move {
        consume_feedback(feedback_shutdown_consumer).await;
    });

    //producer
    let tx_blocking = tx_processed.clone();
    let cycle_clone = Arc::clone(&cycle);

    pool.execute_at_fixed_rate(Duration::from_millis(0), Duration::from_millis(5), move || {
        let mut c = cycle_clone.lock().unwrap();
        if *c > max_cycles {
            //trigger shutdown
            shutdown_notify_producer.notify_waiters();
            return;
        }

        let current_cycle = *c;
        *c += 1;

        let data = generate_sensor_data(current_cycle);
        let mut filters = shared_filters_clone.lock().unwrap();
        let (processed, anomaly) = process_sensor_data(data, &mut filters);

        println!(
            "cycle {:03}, force: {:.2}, anomaly: {}",
            current_cycle, processed.force_data, anomaly
        );

        if let Err(e) = tx_blocking.try_send(processed) {
            eprintln!("Failed to send processed data: {}", e);
        }
    });

    println!("Producer started. Waiting for tasks to complete.");
    //shutdowns
    shutdown_notify.notified().await;
    println!("All cycles processed. Cleaning up...");
    drop(pool);
    drop(tx_processed);              // allow publisher to exit
    feedback_shutdown.notify_waiters(); // allow consumer to exit

    publisher_handle.await.expect("Publisher panicked");
    feedback_handle.await.expect("Feedback panicked");

    println!("Shutdown complete. Exiting.");
}
