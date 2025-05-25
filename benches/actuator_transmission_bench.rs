// benches/rabbitmq_latency.rs
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use lapin::{Connection, ConnectionProperties};
use tokio::runtime::Runtime;
use Real_time_systems_repo::{simulate_actuator, simulate_controller};

fn latency_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function(BenchmarkId::new("Data reception latency", 100), |b| {
        b.to_async(&rt).iter_custom(|_| async {
            let conn =
                Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
                    .await
                    .expect("Connection error");

            let channel = conn.create_channel().await.expect("Channel error");

            let consumer_handle = tokio::spawn(simulate_actuator(channel.clone(), 100));
            simulate_controller(100, channel.clone()).await;
            let _ = consumer_handle.await.unwrap();

            std::time::Duration::from_secs(1)
        });
    });
}

criterion_group!(benches, latency_benchmark);
criterion_main!(benches);
