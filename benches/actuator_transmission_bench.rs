use criterion::{criterion_group, criterion_main, Criterion};
use rand::rng;
use std::hint::black_box;
use std::time::{Duration, Instant};
use Real_time_systems_repo::{simulate_actuator_response, simulate_controller_data}; // Update with actual module paths

fn benchmark_latency(c: &mut Criterion) {
    c.bench_function("latency", |b| {
        b.iter(|| {
            let rng = rng();
            let send_time = Instant::now();

            // Simulate controller sending data with timestamp
            let data = simulate_controller_data(send_time);

            // Simulate actuator processing and reading timestamp
            let recv_time = simulate_actuator_response(black_box(data));

            // Latency calculation
            let latency = recv_time.duration_since(send_time);
            black_box(latency);
        });
    });
}

fn benchmark_throughput(c: &mut Criterion) {
    c.bench_function("throughput", |b| {
        b.iter(|| {
            let mut count = 0;
            let start = Instant::now();
            let duration = Duration::from_secs(1); // 1 second window

            while Instant::now().duration_since(start) < duration {
                let data = simulate_controller_data(Instant::now());
                simulate_actuator_response(black_box(data));
                count += 1;
            }

            let throughput = count as f64 / duration.as_secs_f64();
            black_box(throughput);
        });
    });
}

fn benchmark_jitter(c: &mut Criterion) {
    c.bench_function("jitter", |b| {
        b.iter(|| {
            let mut latencies = vec![];

            for _ in 0..100 {
                let send_time = Instant::now();
                let data = simulate_controller_data(send_time);
                let recv_time = simulate_actuator_response(black_box(data));
                let latency = recv_time.duration_since(send_time);
                latencies.push(latency);
            }

            // Calculate jitter as standard deviation
            let avg =
                latencies.iter().map(|d| d.as_secs_f64()).sum::<f64>() / latencies.len() as f64;
            let jitter = latencies
                .iter()
                .map(|d| {
                    let diff = d.as_secs_f64() - avg;
                    diff * diff
                })
                .sum::<f64>()
                / latencies.len() as f64;

            black_box(jitter.sqrt());
        });
    });
}

criterion_group!(
    benches,
    benchmark_latency,
    benchmark_throughput,
    benchmark_jitter
);
criterion_main!(benches);
