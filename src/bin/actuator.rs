#![allow(unused_imports, unused_variables, unused_mut)]
use futures_util::stream::StreamExt;
use lapin::BasicProperties;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Consumer};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};
use Real_time_systems_repo::{data_structure::*, now_micros};

#[tokio::main]
//start function
pub async fn main() {
    let channel = create_channel().await;

    //thread 1: simulate arm
    consume_sensor_data(channel).await;

    //thread 2: calculate latency
    start_lantency();
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

async fn consume_sensor_data(channel: Channel) {
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

        // Process the sensor data
        control_arm(&channel, sensor_data).await;

        delivery
            .ack(Default::default())
            .await
            .expect("Failed to ack");
    }
}

async fn control_arm(channel: &Channel, mut data: SensorArmData) {
    println!("Executing control for sensor data: {:?}", data);

    // calculate time to reach wrist
    let reach_time = if data.object_data.object_velocity > 0.0 {
        data.object_data.object_distance / data.object_data.object_velocity
    } else {
        0.0
    };

    // move arm to track object
    let horizontal_displacement = data.object_data.object_velocity * reach_time;

    // Target positions
    let target_x = horizontal_displacement;
    let target_y = 0.0; // At arm's level

    // Step 3: Set joint positions to match the predicted wrist position
    // Shoulder affects x, elbow affects y
    data.joints.shoulder_x = target_x;
    data.elbow.elbow_y = target_y;

    // Step 4: Update wrist based on new joint positions
    data.wrist.wrist_x = data.joints.shoulder_x;
    data.wrist.wrist_y = data.elbow.elbow_y;

    println!(
        "Arm moved to track object at predicted (x={}, y={})",
        target_x, target_y
    );

    send_feedback(channel, data).await;
}

/// Simulates sending feedback from actuator to sensor.
pub async fn send_feedback(channel: &Channel, data: SensorArmData) {
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
}

fn start_lantency() {
    // This function is a placeholder for starting latency calculations.
    // In a real application, you would implement the logic to calculate and log latencies here.
    println!("> Starting latency calculations...");
    // For example, you could spawn a new task to periodically log latencies.
    // tokio::spawn(async move { ... });
}
