#![allow(unused_variables)]
use crate::{data_structure::*, now_micros};
use futures_util::stream::StreamExt;
use lapin::BasicProperties;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Consumer};
use rand::Rng;
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};

//start function
pub async fn start() {
    println!("> Actuator is ready to receive sensor data...");

    let mut stamping_data = StampingData::new(0.8, 10, 5);
    let mut channel = init_channel().await;

    let (tx, rx) = tokio::sync::mpsc::channel(100);
    //thread 1 - receiving data
    simulate_actuator(channel, &mut stamping_data).await;

    //thread 2 - calculate reception latency
    start_latency();
}

// This version accepts a mutable StampingData reference
pub async fn init_channel() -> Channel {
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
        .await
        .expect("Connection error");

    // Create a channel
    let channel = conn.create_channel().await.expect("Channel creation error");
    // limit batching and buffering latency
    channel
        .basic_qos(1, BasicQosOptions::default())
        .await
        .expect("Failed to set QoS");

    // Declare the queue (must match the producer queue name)
    channel
        .queue_declare(
            "recieve_sensor_data",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("Queue declaration error");
    channel
}

async fn simulate_actuator(channel: Channel, stamping_data: &mut StampingData) {
    let mut rng = rand::rng();
    // let mut latencies = Vec::new();

    let mut consumer: Consumer = channel
        .basic_consume(
            "sensor_data",
            "actuator_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("Basic consume error");

    while let Some(delivery) = consumer.next().await {
        let delivery = match delivery {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Consumer stream error: {:?}", e);
                continue;
            }
        };

        let payload = &delivery.data;

        let sensor_data: SensorData = match serde_json::from_slice(payload) {
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

        // Update the stamping state
        control_arm(stamping_data, sensor_data).await;

        delivery
            .ack(Default::default())
            .await
            .expect("Failed to ack");

        send_feedback(&channel, "Processed", rng.random::<f64>()).await;
    }
}

/// Simulates sending feedback from actuator to sensor.
pub async fn send_feedback(channel: &Channel, status: &str, adjustment: f64) {
    let feedback = FeedbackData {
        status: status.to_string(),
        adjustment_value: adjustment,
        timestamp: now_micros(),
    };

    let payload = serde_json::to_vec(&feedback).expect("Failed to serialize feedback");

    // Send to feedback queue (sensor listens here)
    channel
        .basic_publish(
            "",
            "feedback_to_sensor", // Sensor should consume from this
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

// #region Control logic
// Control logic that receives deserialized sensor data and updates system state
pub async fn control_arm(stamping_data: &mut StampingData, sensor_data: SensorData) {
    stamping_data.update_sensor_data(sensor_data.position, sensor_data.velocity);

    // Optionally log or simulate behavior based on updated state
    println!(
        "Updated StampingData: pos = {:.3}, vel = {:.3}",
        stamping_data.current_position, stamping_data.current_velocity
    );

    // You can add more predictive or feedback logic here
}

//#endregion
