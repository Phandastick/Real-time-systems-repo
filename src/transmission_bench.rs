use actuator::*;

pub async fn run_controller() {}

pub async fn run_actuator() {
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
