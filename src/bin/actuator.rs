use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Consumer};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};
use Real_time_systems_repo::data_structure::*;

fn now_micros() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros()
}

#[tokio::main]
async fn main() {
    // Connect to RabbitMQ server
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
        .await
        .expect("Connection error");

    // Create a channel
    let channel = conn.create_channel().await.expect("Channel creation error");

    // // limit batching and buffering latency
    // channel
    //     .basic_qos(1, BasicQosOptions::default())
    //     .await
    //     .expect("Failed to set QoS");

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

    // Start consuming messages
    consume_sensor_data(channel).await;
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

        let latency_us = now_micros() - sensor_data.timestamp;
        if latency_us > 1000 {
            eprintln!("ATTENTION!!! Latency > 1ms: {} μs", latency_us);
        } else {
            println!("Latency: {} μs", latency_us);
        }

        // Process the sensor data
        control_arm(sensor_data);

        // Acknowledge message
        delivery
            .ack(Default::default())
            .await
            .expect("Failed to ack");
    }
}

fn control_arm(data: SensorArmData) {
    // Stub for your control logic (PID, predictive controller, etc)
    println!("Executing control for sensor data: {:?}", data);
}
