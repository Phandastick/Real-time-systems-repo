// benches/rabbitmq_latency.rs
use criterion::async_executor::TokioExecutor;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use lapin::{Connection, ConnectionProperties};
use Real_time_systems_repo::simulate_actuator;

fn latency_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Data reception bench");

    group.bench_function(BenchmarkId::new("Data reception bench", 10000), |b| {
        b.to_async(TokioExecutor).iter_custom(|_| async {
            let conn =
                Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
                    .await
                    .expect("Connection error");

            let channel: T<Send> = conn.create_channel().await.expect("Channel error");

            let consumer_handle = tokio::spawn(simulate_actuator(channel));
            // simulate_controller(100, channel.clone()).await;
            let _ = consumer_handle.await.unwrap();

            std::time::Duration::from_secs(1)
        });
    });

    group.finish();
}

criterion_group!(benches, latency_benchmark);
criterion_main!(benches);
