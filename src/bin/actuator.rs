use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use futures_util::stream::StreamExt;
use real_time_systems_repo::data_structure::*;
use serde_json;

#[tokio::main]
async fn main() {
    // Connect to RabbitMQ server
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
        .await.expect("Connection error");

    // Create a channel
    let channel = conn.create_channel().await.expect("Channel creation error");

    // Declare the queue (must match the producer queue name)
    channel.queue_declare(
        "sensor_data",
        QueueDeclareOptions::default(),
        FieldTable::default(),
    )
    .await
    .expect("Queue declaration error");

    // Start consuming from the queue
    let mut consumer = channel.basic_consume(
        "sensor_data",
        "actuator_consumer",
        BasicConsumeOptions::default(),
        FieldTable::default(),
    )
    .await
    .expect("Basic consume error");

    println!("Actuator is ready to receive sensor data...");

    // Consume messages asynchronously
    while let Some(delivery) = consumer.next().await {
        let delivery = delivery.expect("Error in consumer stream");
        let payload = &delivery.data;

        // Deserialize SensorArmData from JSON payload
        let sensor_data: SensorArmData = match serde_json::from_slice(payload) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to deserialize sensor data: {:?}", e);
                delivery.nack(Default::default()).await.expect("Failed to nack");
                continue;
            }
        };

        // Process the sensor data
        control_arm(sensor_data);

        // Acknowledge message so it can be removed from queue
        delivery.ack(Default::default()).await.expect("Failed to ack");
    }
}

fn control_arm(data: SensorArmData) {
    // Stub for your control logic (PID, predictive controller, etc)
    println!("Executing control for sensor data: {:?}", data);
}
