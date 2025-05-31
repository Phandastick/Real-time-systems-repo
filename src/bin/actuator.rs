#![allow(unused_imports, unused_variables, unused_mut)]
use futures_util::stream::StreamExt;
use lapin::BasicProperties;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Consumer};
use serde_json;
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::sync::Barrier;
use Real_time_systems_repo::{data_structure::*, now_micros};

#[tokio::main]
async fn main() {
    start().await;
}

//start function
pub async fn start() {
    let channel = create_channel().await;

    // Set up mpsc channel for latency logging
    let (lat_tx, lat_rx) = mpsc::unbounded_channel();
    let (lat_shoulder_tx, lat_shoulder_rx) = mpsc::unbounded_channel();
    let (lat_elbow_tx, lat_elbow_rx) = mpsc::unbounded_channel();
    let (cycle_tx, cycle_rx) = mpsc::unbounded_channel();
    // channels for joint tasks
    let (shoulder_tx, mut shoulder_rx) = mpsc::unbounded_channel::<ActuatorInstruction>();
    let (elbow_tx, mut elbow_rx) = mpsc::unbounded_channel::<ActuatorInstruction>();

    let sync_barrier = Arc::new(Barrier::new(2)); // Two parties: shoulder and elbow

    // Thread 2: Log latency
    tokio::spawn(start_latency(
        lat_rx,
        lat_elbow_rx,
        lat_shoulder_rx,
        cycle_rx,
    ))
    .await
    .expect("Failed to spawn latency thread");

    //SPAWN SHOULDER JOINT, ELBOW JOINT THREADS and CHANNEL
    // shoudler thread
    let shoulder_barrier = Arc::clone(&sync_barrier);
    tokio::spawn(async move {
        while let Some(pos) = shoulder_rx.recv().await {
            let start_time = now_micros();
            // println!("[SHOULDER] Moving to position: {:?}", pos);
            tokio::time::sleep(tokio::time::Duration::from_micros(pos.time_to_reach)).await; // simulate actuation time
            lat_shoulder_tx
                .send(start_time)
                .expect("Failed to send shoulder latency");
            shoulder_barrier.wait().await; // wait for elbow to finish
        }
    });

    //elbow thread
    let elbow_barrier = Arc::clone(&sync_barrier);
    tokio::spawn(async move {
        while let Some(pos) = elbow_rx.recv().await {
            let start_time = now_micros();
            // println!("[ELBOW] Moving to position: {:?}", pos);
            tokio::time::sleep(tokio::time::Duration::from_micros(pos.time_to_reach)).await; // simulate actuation time
            lat_elbow_tx
                .send(start_time)
                .expect("Failed to send shoulder latency");
            elbow_barrier.wait().await; // wait for shoulder to finish
        }
    });

    // Thread 1: Simulate arm
    let _ = tokio::spawn(consume_sensor_data(
        channel.clone(),
        lat_tx,
        shoulder_tx,
        elbow_tx,
        cycle_tx,
    ))
    .await;
}

async fn create_channel() -> Channel {
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
        .await
        .expect("Connection error");

    // Create a channel
    let channel = conn.create_channel().await.expect("Channel creation error");

    // Set Quality of Service
    channel
        .basic_qos(1, BasicQosOptions::default())
        .await
        .expect("Failed to set QoS");

    // Declare the queue to consume from
    channel
        .queue_declare(
            "sensor_data",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("Queue declaration error");

    channel
}

async fn consume_sensor_data(
    channel: Channel,
    lat_tx: mpsc::UnboundedSender<u128>,
    shoulder_tx: mpsc::UnboundedSender<ActuatorInstruction>,
    elbow_tx: mpsc::UnboundedSender<ActuatorInstruction>,
    cycle_tx: mpsc::UnboundedSender<u128>,
) {
    let mut consumer: Consumer = channel
        .basic_consume(
            "sensor_data",
            "actuator_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("Basic consume error");

    // let mut latencies = Vec::new();
    let mut total_msgs = 0u64;
    let mut missed_deadlines = 0u64;
    let mut cycles = 0;

    println!("> Actuator is ready to receive sensor data...");

    while let Some(delivery) = consumer.next().await {
        cycles += 1;
        let delivery = match delivery {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Consumer stream error: {:?}", e);
                continue;
            }
        };

        let payload = &delivery.data;

        let mut sensor_data: SensorArmData = match serde_json::from_slice(payload) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to deserialize sensor data: {:?}", e);
                delivery
                    .nack(Default::default())
                    .await
                    .expect("Failed to nack");
                continue;
            }
        };

        if cycles < 500 {
            println!("> Warming up, skipping cycle: {}", cycles);
            delivery
                .nack(Default::default())
                .await
                .expect("Failed to nack");
            continue; // skip first 500 cycles - warm up
        }

        lat_tx
            .send(sensor_data.timestamp)
            .expect("Failed to send receive time for latency calculation");

        // cycle starts after receiving data is done
        let cycle_start_time = now_micros();

        // let reception_latency = now_micros().saturating_sub(sensor_data.timestamp);
        // println!("> Reception Latency: {} µs", reception_latency);

        total_msgs += 1;
        println!("> Message count: {:?}", total_msgs);

        let receive_time = now_micros();

        // Process and send response
        control_arm(
            &channel,
            sensor_data,
            receive_time,
            &shoulder_tx,
            &elbow_tx,
            &cycle_tx,
            cycle_start_time,
        )
        .await;

        delivery
            .ack(Default::default())
            .await
            .expect("Failed to ack");
    }
}

async fn control_arm(
    channel: &Channel,
    mut data: SensorArmData,
    receive_time: u128,
    shoulder_tx: &mpsc::UnboundedSender<ActuatorInstruction>,
    elbow_tx: &mpsc::UnboundedSender<ActuatorInstruction>,
    cycle_tx: &mpsc::UnboundedSender<u128>,
    cycle_start_time: u128,
) {
    // println!("Executing control for sensor data: {:?}", data);
    // target never goes negative x
    let mut target_x = data.object_data.object_x;
    let mut target_y = data.object_data.object_y;

    // Arm segment lengths
    let l1 = 3.0; // shoulder to elbow
    let l2 = 3.0; // elbow to wrist

    // Distance to target from shoulder (0, 0)
    let dist = (target_x.powi(2) + target_y.powi(2)).sqrt();

    // Clamp target if beyond max reach
    let (clamped_x, clamped_y) = if dist > l1 + l2 {
        let scale = (l1 + l2) / dist;
        (target_x * scale, target_y * scale)
    } else {
        (target_x, target_y)
    };

    // Inverse kinematics: calculate angles
    let cos_theta2 =
        ((clamped_x.powi(2) + clamped_y.powi(2)) - l1.powi(2) - l2.powi(2)) / (2.0 * l1 * l2);
    let cos_theta2 = cos_theta2.clamp(-1.0, 1.0); // prevent NaNs
    let theta2 = cos_theta2.acos(); // elbow angle

    let k1 = l1 + l2 * theta2.cos();
    let k2 = l2 * theta2.sin();
    let theta1 = clamped_y.atan2(clamped_x) - k2.atan2(k1); // shoulder angle

    // Calculate joint positions
    let shoulder_x = 0.0;
    let shoulder_y = 0.0;

    let elbow_x = shoulder_x + l1 * theta1.cos();
    let elbow_y = shoulder_y + l1 * theta1.sin();

    let wrist_x = elbow_x + l2 * (theta1 + theta2).cos();
    let wrist_y = elbow_y + l2 * (theta1 + theta2).sin();

    // Set new joint and wrist positions
    data.joints.shoulder_x = shoulder_x;
    data.joints.shoulder_y = shoulder_y;

    data.elbow.elbow_x = elbow_x;
    data.elbow.elbow_y = elbow_y;

    data.wrist.wrist_x = wrist_x;
    data.wrist.wrist_y = wrist_y;

    // === NEW: Estimate time until object reaches ground ===
    let object_height = data.object_data.object_height;
    let object_velocity = data.object_data.object_velocity;

    let time_to_reach = if object_velocity > 0.0 {
        (object_height / object_velocity * 1000.00) as u64
    } else {
        println!(
            "[WARNING] Object velocity is zero or negative ({}). Cannot compute time to reach.",
            object_velocity
        );
        0
    };

    // println!("> Estimated time to reach ground: {} µs", time_to_reach);

    //send with time message received to measure latency from message received to actuator execution
    let _ = shoulder_tx.send(ActuatorInstruction {
        x: shoulder_x,
        y: shoulder_y,
        strength: data.arm_strength,
        time_to_reach: time_to_reach,
        timestamp: cycle_start_time,
    });
    let _ = elbow_tx.send(ActuatorInstruction {
        x: elbow_x,
        y: elbow_y,
        strength: data.arm_strength,
        time_to_reach: time_to_reach,
        timestamp: cycle_start_time,
    });

    let compute_done_time = now_micros();
    let arrived_at_ground = compute_done_time + time_to_reach as u128;

    // Internal latency: time spent from receiving to finishing computation
    // let internal_latency = compute_done_time.saturating_sub(receive_time);
    // println!("> Calculation process latency: {} µs", internal_latency);

    send_feedback(
        channel,
        data,
        arrived_at_ground,
        cycle_start_time,
        &cycle_tx,
    )
    .await;
}
/// Simulates sending feedback from actuator to sensor.
pub async fn send_feedback(
    channel: &Channel,
    mut data: SensorArmData,
    arrived_at_ground: u128,
    cycle_start_time: u128,
    cycle_tx: &mpsc::UnboundedSender<u128>,
) {
    // log time done  for feedback AFTER actuator processing
    data.timestamp = now_micros();

    let feedback = data.to_feedback(arrived_at_ground);

    let payload = serde_json::to_vec(&feedback).expect("Failed to serialize feedback");

    channel
        .basic_publish(
            "",
            "feedback_to_sensor", // sensor listens here
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default(),
        )
        .await
        .expect("Failed to publish feedback")
        .await
        .expect("Failed to confirm feedback delivery");

    // println!(
    //     "> Cycle time: {} µs",
    //     now_micros().saturating_sub(cycle_start_time)
    // );
    cycle_tx
        .send(cycle_start_time)
        .expect("Failed to send cycle time for latency calculation");
}

async fn start_latency(
    mut lat_rx: mpsc::UnboundedReceiver<u128>,
    mut lat_elbow_rx: mpsc::UnboundedReceiver<u128>,
    mut lat_shoulder_rx: mpsc::UnboundedReceiver<u128>,
    mut lat_cycle_rx: mpsc::UnboundedReceiver<u128>,
) {
    println!("> Starting latency calculations...");

    // File writer (shared between threads)
    let file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("latency_log.csv")
        .expect("Failed to open latency_log.csv");

    let file = std::sync::Arc::new(std::sync::Mutex::new(csv::Writer::from_writer(file)));

    // Write header only once
    {
        let mut writer = file.lock().unwrap();
        writer
            .write_record(&["timestamp", "latency_type", "latency_μs"])
            .expect("Failed to write CSV header");
        writer.flush().unwrap();
    }

    // Reception latency logging
    {
        let writer = file.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                while let Some(sent_timestamp) = lat_rx.recv().await {
                    let now = now_micros();
                    let latency = now.saturating_sub(sent_timestamp);
                    println!("Data Reception Latency: {} µs", latency);

                    let mut writer = writer.lock().unwrap();
                    writer
                        .write_record(&[
                            now.to_string(),
                            "Data reception latency".to_string(),
                            latency.to_string(),
                        ])
                        .unwrap();
                    writer.flush().unwrap();
                }
            });
        });
    }

    // Cycle latency logging
    {
        let writer = file.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                while let Some(sent_timestamp) = lat_cycle_rx.recv().await {
                    let now = now_micros();
                    let latency = now.saturating_sub(sent_timestamp);
                    println!("Cycle Time: {} µs", latency);

                    let mut writer = writer.lock().unwrap();
                    writer
                        .write_record(&[
                            now.to_string(),
                            "cycle time".to_string(),
                            latency.to_string(),
                        ])
                        .unwrap();
                    writer.flush().unwrap();
                }
            });
        });
    }

    // Elbow latency logging
    {
        let writer = file.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                while let Some(sent_timestamp) = lat_elbow_rx.recv().await {
                    let now = now_micros();
                    let latency = now.saturating_sub(sent_timestamp);
                    println!("Actuator Elbow Latency: {} µs", latency);

                    let mut writer = writer.lock().unwrap();
                    writer
                        .write_record(&[
                            now.to_string(),
                            "elbow actuator time".to_string(),
                            latency.to_string(),
                        ])
                        .unwrap();
                    writer.flush().unwrap();
                }
            });
        });
    }

    // Shoulder latency logging
    {
        let writer = file.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                while let Some(sent_timestamp) = lat_shoulder_rx.recv().await {
                    let now = now_micros();
                    let latency = now.saturating_sub(sent_timestamp);
                    println!("Actuator Shoulder Latency: {} µs", latency);

                    let mut writer = writer.lock().unwrap();
                    writer
                        .write_record(&[
                            now.to_string(),
                            "shoulder actuator time".to_string(),
                            latency.to_string(),
                        ])
                        .unwrap();
                    writer.flush().unwrap();
                }
            });
        });
    }
}
