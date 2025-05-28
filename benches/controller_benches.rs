use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::Arc;
use tokio::{runtime::Runtime, sync::Mutex};

use Real_time_systems_repo::controller_lib::*;

fn bench_generate_sensor_data_fixed(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    //1,000 iterations
    c.bench_function("generate_sensor_data_1k", |b| {
        let shared_feedback = Arc::new(Mutex::new(None));
        b.to_async(&rt).iter_custom(|_iters| async {
            let start = std::time::Instant::now();
            for _ in 0..1000 {
                let result = generate_sensor_data(black_box(10), shared_feedback.clone()).await;
                black_box(result);
            }
            start.elapsed()
        });
    });

    //10,000 iterations
    c.bench_function("generate_sensor_data_10k", |b| {
        let shared_feedback = Arc::new(Mutex::new(None));
        b.to_async(&rt).iter_custom(|_iters| async {
            let start = std::time::Instant::now();
            for _ in 0..10_000 {
                let result = generate_sensor_data(black_box(10), shared_feedback.clone()).await;
                black_box(result);
            }
            start.elapsed()
        });
    });
}

criterion_group!(benches, bench_generate_sensor_data_fixed);
criterion_main!(benches);
