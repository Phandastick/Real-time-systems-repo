use criterion::{criterion_group, criterion_main, Criterion};
use Real_time_systems_repo::{
    actuator_lib::compute_arm_movement,
    controller_lib::generate_sensor_data,
    data_structure::*,
};
use std::sync::Arc;
use tokio::{runtime::Runtime, sync::Mutex};

fn bench_arm_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap(); 

    let shared_feedback = Arc::new(Mutex::new(None));

    c.bench_function("actuator arm computation", |b| {
        b.to_async(&rt).iter(|| async { 
            let sample_data = generate_sensor_data(1, shared_feedback.clone()).await;
            let _ = compute_arm_movement(sample_data);
        });
    });
}

criterion_group!(benches, bench_arm_processing);
criterion_main!(benches);