#![allow(unused_variables)]
use crate::data_structure::*;
use futures_util::stream::StreamExt;
use lapin::BasicProperties;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Consumer};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};

//start function
pub async fn start() {
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

    println!("> Actuator is ready to receive sensor data...");

    let latencies = simulate_actuator(channel).await;
}

fn now_micros() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros()
}

pub async fn simulate_actuator(channel: Channel) -> Vec<u128> {
    let mut consumer: Consumer = channel
        .basic_consume(
            "sensor_data",
            "actuator_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("Basic consume error");

    let mut latencies = Vec::new();

    while let Some(delivery) = consumer.next().await {
        let delivery = match delivery {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Consumer stream error: {:?}", e);
                continue;
            }
        };

        let payload = &delivery.data;

        // handle deserialize errors
        let sensor_data: SensorArmData = match serde_json::from_slice(payload) {
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

        let latency = now_micros() - sensor_data.timestamp;
        latencies.push(latency);

        delivery.ack(Default::default()).await.expect("Ack failed");

        send_feedback(
            &channel,
            sensor_data.actuator_id,
            "Processed",
            sensor_data.force_data,
        )
        .await;
    }

    latencies //return vector
}

/// Simulates sending feedback from actuator to sensor.
pub async fn send_feedback(channel: &Channel, actuator_id: u32, status: &str, adjustment: f64) {
    let feedback = FeedbackData {
        actuator_id,
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
