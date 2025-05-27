#![allow(unused_imports, unused_variables, unused_mut)]
use futures_util::stream::StreamExt;
use lapin::BasicProperties;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Consumer};
use serde_json;
use std::f32::consts::PI;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
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

    // Thread 2: Log latency
    tokio::spawn(start_latency(lat_rx));

    // Thread 1: Simulate arm
    let _ = tokio::spawn(consume_sensor_data(channel.clone(), lat_tx)).await;
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

async fn consume_sensor_data(channel: Channel, lat_tx: mpsc::UnboundedSender<u128>) {
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

    println!("> Actuator is ready to receive sensor data...");

    while let Some(delivery) = consumer.next().await {
        let cycle_start_time = now_micros();

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

        total_msgs += 1;
        // lat_tx
        //     .send(sensor_data.timestamp)
        //     .await
        //     .expect("Failed to send receive time for latency calculation");
        let receive_time = now_micros();

        // Process and send response
        control_arm(
            &channel,
            sensor_data,
            receive_time,
            &shoulder_tx,
            &elbow_tx,
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
    shoulder_tx: &mpsc::UnboundedSender<ShoulderData>,
    elbow_tx: &mpsc::UnboundedSender<ElbowData>,
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

    // Timestamp when computation ends
    let compute_done_time = now_micros();

    data.timestamp = compute_done_time;

    // Internal latency: time spent from receiving to finishing computation
    let internal_latency = compute_done_time.saturating_sub(receive_time);
    println!(
        ">> Internal actuator processing latency: {} µs",
        internal_latency
    );

    println!(
        "Arm moved to catch object at target (x={}, y={}) with elbow at ({:.2}, {:.2}) and wrist at ({:.2}, {:.2})",
        target_x, target_y, elbow_x, elbow_y, wrist_x, wrist_y
    );

    send_feedback(channel, data, cycle_start_time).await;
}
/// Simulates sending feedback from actuator to sensor.
pub async fn send_feedback(channel: &Channel, mut data: SensorArmData, cycle_start_time: u128) {
    // log time done  for feedback AFTER actuator processing
    data.timestamp = now_micros();

    let feedback = data.to_feedback();

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

    println!("> Sent feedback to sensor: {:?}", feedback);
    println!(
        "> Cycle time: {} µs",
        now_micros().saturating_sub(cycle_start_time)
    );
}

async fn start_latency(mut lat_rx: mpsc::UnboundedReceiver<u128>) {
    println!("> Starting latency calculations...");

    while let Some(sent_timestamp) = lat_rx.recv().await {
        let now = now_micros();
        let latency = now.saturating_sub(sent_timestamp);
        println!("Latency: {} µs", latency);
    }
}
