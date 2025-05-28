use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

use Real_time_systems_repo::{controller_lib::*, data_structure::*};  // adjust your crate name and imports

fn bench_generate_sensor_data(c: &mut Criterion) {
    // Create a Tokio runtime for async code
    let rt = Runtime::new().unwrap();

    // Shared state used in generate_sensor_data
    let shared_feedback = Arc::new(Mutex::new(None));

    c.bench_function("generate_sensor_data", |b| {
        b.to_async(&rt).iter(|| async {
            // Await the future and wrap the result in black_box to avoid optimizations
            let result = generate_sensor_data(black_box(1), shared_feedback.clone()).await;
            black_box(result);
        });
    });
}

criterion_group!(benches, bench_generate_sensor_data);
criterion_main!(benches);
