use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Consumer};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};
use Real_time_systems_repo::{data_structure::*, lib::now_micros};

const DEADLINE_US: u128 = 1000; // 1 ms deadline

#[tokio::main]
async fn main() {
    // Connect to RabbitMQ server
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

    let mut latencies = Vec::new();
    let mut total_msgs = 0u64;
    let mut missed_deadlines = 0u64;

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
		// {
		// 	let latency_us = now_micros() - sensor_data.timestamp;
		// 	total_msgs += 1;
	
		// 	// log missed deadlines
		// 	if latency_us > DEADLINE_US {
		// 		missed_deadlines += 1;
		// 		eprintln!(
		// 			"ATTENTION!!! Latency > {} μs: {} μs",
		// 			DEADLINE_US, latency_us
		// 		);
		// 	} else {
		// 		println!("Latency: {} μs", latency_us);
		// 	}
	
		// 	latencies.push(latency_us);
	
		// 	if total_msgs % 20 == 0 {
		// 		let min = *latencies.iter().min().unwrap();
		// 		let max = *latencies.iter().max().unwrap();
		// 		let avg = latencies.iter().sum::<u128>() as f64 / latencies.len() as f64;
		// 		let missed_ratio = missed_deadlines as f64 / total_msgs as f64 * 100.0;
		// 		println!(
		// 			"Latency over {} msgs: min={}μs max={}μs avg={:.2}μs missed_deadline_ratio={:.2}%",
		// 			total_msgs, min, max, avg, missed_ratio
		// 		);
		// 	}
		}

        

        // Process the sensor data
        control_arm(sensor_data);

        delivery
            .ack(Default::default())
            .await
            .expect("Failed to ack");
    }
}

fn control_arm(data: SensorArmData) {
    println!("Executing control for sensor data: {:?}", data);
}
