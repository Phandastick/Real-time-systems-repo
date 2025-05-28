use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::Arc;
use tokio::{runtime::Runtime, sync::Mutex};
use std::time::Instant;

use Real_time_systems_repo::controller_lib::{
    generate_sensor_data,
    process_sensor_data,
    detect_anomaly,
    generate_anomalous_object_data,
};
use Real_time_systems_repo::data_structure::{FeedbackData, Filters,};

fn bench_generate_sensor_data(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("generate_sensor_data", |b| {
        let shared_feedback = Arc::new(Mutex::new(None));
        b.to_async(&rt).iter_custom(|iters| {
            let shared_feedback = shared_feedback.clone();
            async move {
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let result = generate_sensor_data(black_box(10), shared_feedback.clone()).await;
                    black_box(result);
                }
                start.elapsed()
            }
        });
    });
}

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

criterion_group!(
    benches,
    bench_generate_sensor_data,
    // bench_process_sensor_data,
    // bench_detect_anomaly,
    // bench_generate_anomalous_object_data
);
criterion_main!(benches);
