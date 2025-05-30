use criterion::{criterion_group, criterion_main, Criterion};
use tokio::sync::mpsc;
use std::hint::black_box;
use std::sync::Arc;
use tokio::{runtime::Runtime, sync::Mutex};
use std::time::{Instant, Duration};

use Real_time_systems_repo::controller_lib::{
    generate_sensor_data,
    process_sensor_data,
    detect_anomaly,
    generate_anomalous_object_data,
    publish,
};
use Real_time_systems_repo::data_structure::{FeedbackData, Filters};
use lapin::{
    options::{BasicPublishOptions, QueueDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
};
use serde::{Serialize, Deserialize};

//no blackbox
// fn bench_generate_sensor_data(c: &mut Criterion) {
//     let rt = Runtime::new().unwrap();

//     c.bench_function("generate_sensor_data_noblackbox", |b| {
//         let shared_feedback = Arc::new(Mutex::new(None));
//         b.to_async(&rt).iter_custom(|iters| {
//             let shared_feedback = shared_feedback.clone();
//             async move {
//                 let start = std::time::Instant::now();
//                 for _ in 0..iters {
//                     // Direct call without black_box
//                     let _ = generate_sensor_data(10, shared_feedback.clone()).await;
//                 }
//                 start.elapsed()
//             }
//         });
//     });
// }
fn bench_sensor_generation_interval(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("sensor_data_generation_at_interval_proof", |b| {
        let shared_feedback = Arc::new(Mutex::new(None));
        let shared_filters = Arc::new(Mutex::new(Filters::new()));

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // Drain receiver concurrently to prevent blocking
        let drain_handle = rt.spawn(async move {
            while let Some(_msg) = rx.recv().await {
                // consume the messages
            }
        });

        b.to_async(&rt).iter_custom(|iters| {
            let shared_feedback = shared_feedback.clone();
            let shared_filters = shared_filters.clone();
            let tx = tx.clone();

            async move {
                let start = Instant::now();

                for i in 0..iters {
                    let data = generate_sensor_data(i as u64, shared_feedback.clone()).await;

                    let mut filters = shared_filters.lock().await;
                    let (processed, anomaly) = process_sensor_data(data, &mut filters);

                    if anomaly {
                        filters.reset();
                    } else {
                        if tx.send(processed).await.is_err() {
                            panic!("Receiver dropped");
                        }
                    }
                }

                start.elapsed()
            }
        });
        rt.block_on(async {
            drain_handle.abort();
        });
    });
}


// fn bench_send_sensor_data(c: &mut Criterion) {
//     let rt = Runtime::new().unwrap();

//     c.bench_function("sensor_to_rabbitMQ_sending", |b| {
//         let shared_feedback = Arc::new(Mutex::new(None));

//         b.to_async(&rt).iter_custom(|iters| {
//             let shared_feedback = shared_feedback.clone();

//             async move {
//                 // Establish connection and declare queue once outside loop
//                 let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
//                     .await
//                     .expect("Connection error");
//                 let channel = conn.create_channel().await.expect("Channel creation error");
//                 channel.queue_declare(
//                     "sensor_data",
//                     QueueDeclareOptions::default(),
//                     FieldTable::default(),
//                 )
//                 .await
//                 .expect("Queue declaration error");
                
                
//                 // Generate sensor data
//                 let result = generate_sensor_data(black_box(10), shared_feedback.clone()).await;
//                 black_box(&result);
//                 let start = Instant::now();
//                 //loop starts here
//                 for _ in 0..iters {
//                     // Publish to RabbitMQ once per iteration
//                     publish(&channel, &result).await.expect("Failed to publish");
//                     black_box(&result);
//                 }

//                 start.elapsed()
//             }
//         });
//     });
// }

// fn bench_process_sensor_data(c: &mut Criterion) {
//     let rt = Runtime::new().unwrap();
//     let shared_feedback = Arc::new(Mutex::new(None::<FeedbackData>));
//     let data = rt.block_on(generate_sensor_data(1, shared_feedback));

//     c.bench_function("process_sensor_data", |b| {
//         b.iter_custom(|iters| {
//             let start = Instant::now();
//             for _ in 0..iters {
//                 let mut filters = Filters::new();
//                 let processed = process_sensor_data(black_box(data.clone()), &mut filters);
//                 black_box(processed);
//             }
//             start.elapsed()
//         });
//     });
// }

// fn bench_detect_anomaly(c: &mut Criterion) {
//     c.bench_function("detect_anomaly", |b| {
//         b.iter_custom(|iters| {
//             let start = Instant::now();
//             for _ in 0..iters {
//                 let result = detect_anomaly(black_box(51.0), black_box(0.0), black_box(50.0));
//                 black_box(result);
//             }
//             start.elapsed()
//         });
//     });
// }

// fn bench_generate_anomalous_object_data(c: &mut Criterion) {
//     c.bench_function("generate_anomalous_object_data", |b| {
//         b.iter_custom(|iters| {
//             let start = Instant::now();
//             for _ in 0..iters {
//                 let obj = generate_anomalous_object_data();
//                 black_box(obj);
//             }
//             start.elapsed()
//         });
//     });
// }

// criterion_group!(
//     benches,
//     bench_generate_sensor_data,
//     // bench_process_sensor_data,
//     // bench_detect_anomaly,
//     // bench_generate_anomalous_object_data
// );
// criterion_main!(benches);
criterion_group!(
    benches,
    bench_sensor_generation_interval,
);
criterion_main!(benches);
