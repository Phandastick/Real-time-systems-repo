use criterion::{criterion_group, criterion_main, Criterion};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use Real_time_systems_repo::Command;

fn latency_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("controller->actuator latency", |b| {
        b.to_async(&rt).iter(|| async {
            let (tx, mut rx) = mpsc::channel::<Command>(100);

            // Simulate actuator task
            let actuator = tokio::spawn(async move {
                while let Some(cmd) = rx.recv().await {
                    let latency = cmd.timestamp.elapsed();
                    println!("Latency: {:?}", latency); // Or log to a vector/file
                }
            });

            // Simulate controller sending messages
            for id in 0..100 {
                let cmd = Command {
                    id,
                    timestamp: Instant::now(),
                };
                tx.send(cmd).await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await; // Simulate real-time pacing
            }

            actuator.await.unwrap();
        });
    });
}

criterion_group!(benches, latency_benchmark);
criterion_main!(benches);
