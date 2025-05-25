use crate::data_structure::*;
use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Channel, Consumer};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_micros() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros()
}

pub async fn simulate_actuator(channel: Channel, expected_messages: usize) -> Vec<u128> {
    let mut consumer: Consumer = channel
        .basic_consume(
            "sensor_data",
            "actuator_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("Basic consume error");

    let mut latencies = Vec::with_capacity(expected_messages);

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

        if latencies.len() >= expected_messages {
            break;
        }
    }

    latencies //return vector
}
