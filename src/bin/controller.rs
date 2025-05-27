use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use rand::random;
use scheduled_thread_pool::ScheduledThreadPool;
use serde_json;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc;
use tokio::sync::Notify;
use Real_time_systems_repo::data_structure::*;

const WINDOW_SIZE: usize = 5;

#[derive(Debug, Clone)]
struct MovingAverage {
    buffer: [f32; WINDOW_SIZE],
    index: usize,
    sum: f32,
    count: usize,
}

impl MovingAverage {
    fn new() -> Self {
        Self {
            buffer: [0.0; WINDOW_SIZE],
            index: 0,
            sum: 0.0,
            count: 0,
        }
    }

    fn update(&mut self, val: f32) -> f32 {
        if self.count < WINDOW_SIZE {
            self.count += 1;
        } else {
            self.sum -= self.buffer[self.index];
        }
        self.buffer[self.index] = val;
        self.sum += val;
        self.index = (self.index + 1) % WINDOW_SIZE;
        self.sum / self.count as f32
    }
}

fn now_micros() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros()
}

#[derive(Clone)]
struct Filters {
    wrist_x_filter: MovingAverage,
    wrist_y_filter: MovingAverage,
    shoulder_x_filter: MovingAverage,
    shoulder_y_filter: MovingAverage,
    elbow_x_filter: MovingAverage,
    elbow_y_filter: MovingAverage,
    arm_velocity_filter: MovingAverage,
    object_distance_x_filter: MovingAverage,
    object_distance_y_filter: MovingAverage,
}

impl Filters {
    fn new() -> Self {
        Self {
            wrist_x_filter: MovingAverage::new(),
            wrist_y_filter: MovingAverage::new(),
            shoulder_x_filter: MovingAverage::new(),
            shoulder_y_filter: MovingAverage::new(),
            elbow_x_filter: MovingAverage::new(),
            elbow_y_filter: MovingAverage::new(),
            arm_velocity_filter: MovingAverage::new(),
            object_distance_x_filter: MovingAverage::new(),
            object_distance_y_filter: MovingAverage::new(),
        }
    }
}

fn detect_anomaly(value: f32, lower: f32, upper: f32) -> bool {
    value < lower || value > upper
}

fn generate_anomalous_object_data() -> ObjectData {
    ObjectData {
        // velocity is very low or 0, indicating no drop or static obstruction like a hand
        object_velocity: random::<f32>() * 1.0, // 0–1 m/s (very slow or static)

        // since mass can change, the heavier the object the more difficult it is to catch as there is more momentum thus more velocity
        // anomaly: very heavy (hand = 400–600g vs normal 1–5g)
        object_mass: 100.0 + random::<f32>() * 500.0, // 100g–600g = unexpected

        // assume size is variable in small range of 4-5l
        // anomaly: either too small or much larger object
        object_size: 10.0 + random::<f32>() * 20.0, // 10–30L = abnormal for expected object

        // distance changes due to the object being let go at different areas of the tube, where tube is a circle with diameter of 3cm
        // anomaly: object is placed very close or far off, like a hand waving or blocking the tube
        object_distance_x: 2.5 + random::<f32>() * 3.0, // 2.5–5.5 cm (outside normal tube center)

        // -1.5 to 1.5, but hand might extend further
        object_distance_y: -2.0 + random::<f32>() * 4.0, // -2.0 to +2.0 (possibly out of vertical bounds)
    }
}

fn generate_sensor_data(cycle: u64) -> SensorArmData {
    let object_data = if cycle % 10 == 0 {
        // Every 10th cycle, simulate an anomaly (like hand)
        generate_anomalous_object_data()
    } else {
        // Normal falling object
        ObjectData {
            //velocity > mass since v = u + at, where v = final velocity, u = initial velocity(at rest so 0), a = acceleration (gravity 9.8), t = time (object has to be caught at 1s)
            // up to 11.8 m/s and the object can be heavier than 1g
            object_velocity: 9.8 + random::<f32>() * 2.0, 
            //since mass can change, the heavier the object the more difficult it is to catch as there is more momentum thus more velocity
            //thus variability in velocity is needed
            //1 - 5g
            object_mass: 1.0 + random::<f32>() * 4.0,
            //assume size is variable in small range of 4-5l
            object_size: 4.0 + random::<f32>(),
            //distance changes due to the object being let go at different areas of the tube, where tube is a circle with diameter of 3cm
            //thus object is at any point within the tube(circle)
            //need x and y to tell where the robotic arm is in relation to the object to catch it
            //where x is front back y is left right
            //max object distance is 3cm(diameter of tube) for x and y so 4cm is a good range if accounting for some wind 
            //range of 0-3 in addition to max length of arm
            object_distance_x: random::<f32>() * 3.0,
            //-1.5 to 1.5
            object_distance_y: (random::<f32>() * 3.0) - 1.5,
            //requires arm data first, so we set it to 0
            object_height: 0.0, 
        }
fn generate_sensor_data(cycle: u64) -> SensorArmData {
    let object_data = ObjectData {
        //velocity > mass since v = u + at, where v = final velocity, u = initial velocity(at rest so 0), a = acceleration (gravity 9.8), t = time (object has to be caught at 1s)
        // up to 11.8 m/s and the object can be heavier than 1g
        object_velocity: 9.8 + random::<f32>() * 2.0,
        //since mass can change, the heavier the object the more difficult it is to catch as there is more momentum thus more velocity
        //thus variability in velocity is needed
        //1 - 5g
        object_mass: 1.0 + random::<f32>() * 4.0,
        //assume size is variable in small range of 4-5l
        object_size: 4.0 + random::<f32>(),
        //distance changes due to the object being let go at different areas of the tube, where tube is a circle with diameter of 3cm
        //thus object is at any point within the tube(circle)
        //need x and y to tell where the robotic arm is in relation to the object to catch it
        //max object distance is 3cm(diameter of tube) for x and y so 4cm is a good range if accounting for some wind
        //range of 0-3 in addition to max length of arm
        object_distance_x: random::<f32>() * 3.0,
        //-1.5 to 1.5
        object_distance_y: (random::<f32>() * 3.0) - 1.5,
    };

    let mut sensor_data = SensorArmData::new(object_data.clone());
    sensor_data.update_object_data(object_data);

    //using forward kinematics to calculate arm positions

    //wrist length > elbow length > shoulder length
    //shoulder is the base of the arm, so it is the least variable
    //shoulder length can vary from 0cm to 1cm
    sensor_data.joints.shoulder_x = random::<f32>() * 1.0;
    sensor_data.joints.shoulder_y = (random::<f32>() * 3.0) - 1.5; // y: [-1.5, 1.5]

    //realistic segment lengths (upper and lower arm)
    //l1 = shoulder to elbow (1–4cm), l2 = elbow to wrist (4–7cm)
    let l1 = 1.0 + random::<f32>() * 3.0; // 1cm base + 0–3cm range = 1–4cm
    let l2 = 4.0 + random::<f32>() * 3.0; // 4–7cm

    //limit shoulder angle to forward-facing only, so wrist stays in x ≥ 0
    //angle from 0 (right) to π (left), but we clamp it to [0, π/2] for safe forward-right region
    let theta1 = random::<f32>() * std::f32::consts::FRAC_PI_2; // [0, π/2]

    //elbow bend ±90°, so -π/2 to π/2 range is OK
    let theta2 = (random::<f32>() - 0.5) * std::f32::consts::PI;

    //using FK to get elbow position from shoulder + angle + l1
    //this models the upper arm segment
    sensor_data.elbow.elbow_x = sensor_data.joints.shoulder_x + l1 * theta1.cos();
    sensor_data.elbow.elbow_y = sensor_data.joints.shoulder_y + l1 * theta1.sin();
    sensor_data.elbow.elbow_y = sensor_data.elbow.elbow_y.clamp(-1.5, 1.5); // constrain y range

    //wrist is the end of the forearm, which bends at the elbow
    //direction is based on total angle (shoulder + elbow joint)
    sensor_data.wrist.wrist_x = sensor_data.elbow.elbow_x + l2 * (theta1 + theta2).cos();
    sensor_data.wrist.wrist_y = sensor_data.elbow.elbow_y + l2 * (theta1 + theta2).sin();

    // clamp wrist_x to ≥ shoulder_x
    if sensor_data.wrist.wrist_x < sensor_data.joints.shoulder_x {
        sensor_data.wrist.wrist_x = sensor_data.joints.shoulder_x;
    }

    // clamp wrist_y to [-1.5, 1.5]
    sensor_data.wrist.wrist_y = sensor_data.wrist.wrist_y.clamp(-1.5, 1.5);

    //suggested arm velocity to catch object
    sensor_data.arm_velocity = random::<f32>() * 10.0;

    //arm strength is a crude estimate based on F = m * a,
    //assuming velocity is proportional to acceleration here
    sensor_data.arm_strength = sensor_data.arm_velocity * sensor_data.object_data.object_mass;

    sensor_data.timestamp = now_micros();
    sensor_data.object_data.object_height = shoulder_y + l1 * sin(theta1) + l2 * sin(theta1 + theta2);
    sensor_data
}

fn process_sensor_data(raw: SensorArmData, filters: &mut Filters) -> (SensorArmData, bool) {
    let mut filtered = raw.clone();

    filtered.wrist.wrist_x = filters.wrist_x_filter.update(raw.wrist.wrist_x);
    filtered.wrist.wrist_y = filters.wrist_y_filter.update(raw.wrist.wrist_y);
    filtered.joints.shoulder_x = filters.shoulder_x_filter.update(raw.joints.shoulder_x);
    filtered.joints.shoulder_y = filters.shoulder_y_filter.update(raw.joints.shoulder_y);
    filtered.elbow.elbow_x = filters.elbow_x_filter.update(raw.elbow.elbow_x);
    filtered.elbow.elbow_y = filters.elbow_y_filter.update(raw.elbow.elbow_y);
    filtered.arm_velocity = filters.arm_velocity_filter.update(raw.arm_velocity);
    filtered.object_data.object_distance_x = filters
        .object_distance_x_filter
        .update(raw.object_data.object_distance_x);
    filtered.object_data.object_distance_y = filters
        .object_distance_y_filter
        .update(raw.object_data.object_distance_y);
    filtered.arm_strength = filtered.arm_velocity * raw.object_data.object_mass;

    let anomaly = detect_anomaly(filtered.arm_strength, 0.0, 50.0)
        || detect_anomaly(filtered.wrist.wrist_x, 0.0, 12.0)
        || detect_anomaly(filtered.wrist.wrist_y, -1.5, 1.5)
        || detect_anomaly(filtered.joints.shoulder_x, 0.0, 1.0)
        || detect_anomaly(filtered.joints.shoulder_y, -1.5, 1.5)
        || detect_anomaly(filtered.elbow.elbow_x, 0.0, 12.0)
        || detect_anomaly(filtered.elbow.elbow_y, -1.5, 1.5)
        || detect_anomaly(filtered.object_data.object_mass, 1.0, 5.0)             // extra mass
        || detect_anomaly(filtered.object_data.object_size, 4.0, 5.0)             // unusual size
        || detect_anomaly(filtered.object_data.object_velocity, 9.8, 11.8); // non-moving object

    (filtered, anomaly)
}

async fn consume_feedback(shutdown: Arc<Notify>) {
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
        .await
        .expect("Connection error");

    let channel = conn.create_channel().await.expect("Channel creation error");

    channel
        .queue_declare(
            "feedback_to_sensor",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("Queue declaration error");

    let mut consumer = channel
        .basic_consume(
            "feedback_to_sensor",
            "feedback_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("Basic consume error");

    println!("> Feedback consumer ready...");

    loop {
        tokio::select! {
            maybe_delivery = consumer.next() => {
                if let Some(Ok(delivery)) = maybe_delivery {
                    let payload = &delivery.data;
                    let feedback: FeedbackData = match serde_json::from_slice(payload) {
                        Ok(fb) => fb,
                        Err(e) => {
                            eprintln!("Failed to deserialize feedback: {:?}", e);
                            delivery.nack(Default::default()).await.expect("Failed to nack");
                            continue;
                        }
                    };
                    println!("Received feedback: {:?}", feedback);
                    delivery.ack(Default::default()).await.expect("Failed to ack");
                } else {
                    break;
                }
            }
            _ = shutdown.notified() => {
                println!("Feedback consumer received shutdown signal.");
                break;
            }
        }
    }

    println!("Feedback consumer exiting cleanly.");
}

#[tokio::main]
async fn main() {
    let pool = ScheduledThreadPool::new(4);
    let cycle = Arc::new(Mutex::new(1u64));
    let max_cycles = 10u64;
    let shared_filters = Arc::new(Mutex::new(Filters::new()));
    let shared_filters_clone = Arc::clone(&shared_filters);
    let (tx_processed, mut rx_processed) = mpsc::channel::<SensorArmData>(100);
    let tx_blocking = tx_processed.clone();
    let cycle_clone = Arc::clone(&cycle);
    let shutdown_notify = Arc::new(Notify::new());
    let shutdown_notify_producer = Arc::clone(&shutdown_notify);
    let feedback_shutdown = Arc::new(Notify::new());
    let feedback_shutdown_consumer = Arc::clone(&feedback_shutdown);

    pool.execute_at_fixed_rate(Duration::from_millis(0), Duration::from_millis(10), move || {
        let mut c = cycle_clone.lock().unwrap();
        if *c > max_cycles {
            shutdown_notify_producer.notify_waiters();
            return;
        }

        let current_cycle = *c;
        *c += 1;

        let data = generate_sensor_data(current_cycle);
        let mut filters = shared_filters_clone.lock().unwrap();
        let (processed, anomaly) = process_sensor_data(data, &mut filters);

        if anomaly {
            // Only log anomaly, don't send data
            println!("Anomaly detected in cycle {}: {:?}", current_cycle, processed);
        } else {
            // No anomaly: send data
            println!(
                "cycle {:03}, arm_strength: {:.2}, anomaly: {}",
                current_cycle, processed.arm_strength, anomaly
            );

            if let Err(e) = tx_blocking.try_send(processed) {
                eprintln!("Failed to send processed data: {}", e);
            }
        }
    });

    let publisher_handle = tokio::spawn(async move {
        let conn =
            Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
                .await
                .expect("Connection error");
        let channel = conn.create_channel().await.expect("Channel creation error");

        channel
            .queue_declare(
                "sensor_data",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("Queue declaration error");

        while let Some(processed_data) = rx_processed.recv().await {
            let payload = serde_json::to_vec(&processed_data).expect("Serialization failed");

            channel
                .basic_publish(
                    "",
                    "sensor_data",
                    BasicPublishOptions::default(),
                    &payload,
                    Default::default(),
                )
                .await
                .expect("Publish failed")
                .await
                .expect("Confirmation failed");
        }
        println!("Publisher exiting cleanly.");
    });

    let feedback_handle = tokio::spawn(async move {
        consume_feedback(feedback_shutdown_consumer).await;
    });

    let tx_blocking = tx_processed.clone();
    let cycle_clone = Arc::clone(&cycle);

    pool.execute_at_fixed_rate(
        Duration::from_millis(0),
        Duration::from_millis(10),
        move || {
            let mut c = cycle_clone.lock().unwrap();
            if *c > max_cycles {
                shutdown_notify_producer.notify_waiters();
                return;
            }

            let current_cycle = *c;
            *c += 1;

            let data = generate_sensor_data(current_cycle);
            let mut filters = shared_filters_clone.lock().unwrap();
            let (processed, anomaly) = process_sensor_data(data, &mut filters);

            println!(
                "cycle {:03}, arm_strength: {:.2}, anomaly: {}",
                current_cycle, processed.arm_strength, anomaly
            );

            if let Err(e) = tx_blocking.try_send(processed) {
                eprintln!("Failed to send processed data: {}", e);
            }
        },
    );

    shutdown_notify.notified().await;
    println!("All cycles processed. Cleaning up...");
    drop(pool);
    drop(tx_processed);
    feedback_shutdown.notify_waiters();

    publisher_handle.await.expect("Publisher panicked");
    feedback_handle.await.expect("Feedback panicked");
    println!("Shutdown complete. Exiting.");
}
