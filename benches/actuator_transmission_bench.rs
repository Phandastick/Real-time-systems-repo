use criterion::{criterion_group, criterion_main, Criterion};
use Real_time_systems_repo::{
    actuator_lib::compute_arm_movement, controller_lib::generate_sensor_data, data_structure::*,
};

fn bench_arm_processing(c: &mut Criterion) {
    // Sample SensorArmData (could randomize this for multiple runs)

    c.bench_function("actuator arm computation", |b| {
        b.iter(|| {
            let sample_data = generate_sensor_data(1);
            let _ = compute_arm_movement(sample_data.clone());
        });
    });
}

criterion_group!(benches, bench_arm_processing);
criterion_main!(benches);
