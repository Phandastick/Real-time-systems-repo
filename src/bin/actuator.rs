#![allow(unused_imports, unused_variables, unused_mut)]
use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties, Consumer};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};
use Real_time_systems_repo::{data_structure::*, now_micros};

#[tokio::main]
async fn main() {
    // Connect to RabbitMQ server
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

    println!("> Actuator is ready to receive sensor data...");

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

    while let Some(delivery) = consumer.next().await {
        let delivery = match delivery {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Consumer stream error: {:?}", e);
                continue;
            }
        };

        let payload = &delivery.data;

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

        // Process the sensor data
        control_arm(&channel, sensor_data).await;

        delivery
            .ack(Default::default())
            .await
            .expect("Failed to ack");
    }
}

async fn control_arm(channel: &Channel, data: SensorArmData) {
    println!("Executing control for sensor data: {:?}", data);

    // Adjust wrist, joints, elbow based on sensor data (example logic)
    let mut adjusted_wrist = data.wrist.clone();
    let mut adjusted_joints = data.joints.clone();
    let mut adjusted_elbow = data.elbow.clone();

    if data.arm_strength > 10.0 {
        // Example adjustment for strong arm
        adjusted_wrist.wrist_x *= 0.9;
    } else if data.arm_strength < 2.0 {
        // Example adjustment for weak arm
        adjusted_wrist.wrist_x *= 1.1;
    }

    send_feedback(channel, adjusted_wrist, adjusted_joints, adjusted_elbow).await;
}


pub async fn send_feedback(channel: &Channel, wrist: WristData, joints: ShoulderData, elbow: ElbowData) {
    let feedback = FeedbackData {
        wrist,
        joints,
        elbow,
        timestamp: now_micros(),
    };

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

