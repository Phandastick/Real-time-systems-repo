use std::{
    sync::{
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures_util::stream::StreamExt;
use lapin::{
    options::*,
    types::FieldTable,
    Connection,
    ConnectionProperties,
};
use serde::Serialize;
use serde_json;
use tokio::sync::{mpsc, Mutex, Notify};
use fastrand;
use crate::data_structure::*;


pub fn now_micros() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros()
}

pub fn detect_anomaly(value: f32, lower: f32, upper: f32) -> bool {
    value < lower || value > upper
}

pub fn generate_anomalous_object_data() -> ObjectData {
    ObjectData {
        // velocity is very low or 0, indicating no drop or static obstruction like a hand
        object_velocity: fastrand::f32() * 1.0, // 0–1 m/s (very slow or static)

        // since mass can change, the heavier the object the more difficult it is to catch as there is more momentum thus more velocity
        // anomaly: very heavy (hand = 400–600g vs normal 1–5g)
        object_mass: 100.0 + fastrand::f32() * 500.0, // 100g–600g = unexpected

        // assume size is variable in small range of 4-5l
        // anomaly: either too small or much larger object
        object_size: 10.0 + fastrand::f32() * 20.0, // 10–30L = abnormal for expected object

        // distance changes due to the object being let go at different areas of the tube, where tube is a circle with diameter of 3cm
        // anomaly: object is placed very close or far off, like a hand waving or blocking the tube
        object_x: 7.0 + fastrand::f32() * 3.0, 

        object_y: 7.0 + fastrand::f32() * 4.0,

        object_height: 0.0,
    }
}

pub fn generate_normal_object_data() -> ObjectData {
    // Normal falling object
    ObjectData {
        //velocity > mass since v = u + at, where v = final velocity, u = initial velocity(at rest so 0), a = acceleration (gravity 9.8), t = time (object has to be caught at 1s)
        // up to 11.8 m/s and the object can be heavier than 1g
        object_velocity: 9.8 + fastrand::f32() * 2.0,
        //since mass can change, the heavier the object the more difficult it is to catch as there is more momentum thus more velocity
        //thus variability in velocity is needed
        //1 - 5g
        object_mass: 1.0 + fastrand::f32() * 4.0,
        //assume size is variable in small range of 4-5l
        object_size: 4.0 + fastrand::f32(),
        //distance changes due to the object being let go at different areas of the tube, where tube is a circle with diameter of 3cm
        //thus object is at any point within the tube(circle)
        //need x and y to tell where the robotic arm is in relation to the object to catch it
        //where x is front back y is left right
        //max object distance is 3cm(diameter of tube) for x and y so 4cm is a good range if accounting for some wind 
        //tube infront by 4cm, assume tube is 3cm in diameter
        object_x: 4.0 + fastrand::f32() * 3.0,
        //-5 to 5
        object_y: (fastrand::f32() * 10.0) - 5.0,
        //object height is the distance from the top of the tube to the object
        //calculated later based on arm
        object_height: 0.0,
    }
}


fn branchless_clamp(val: f32, min: f32, max: f32) -> f32 {
    val.max(min).min(max)
}

pub async fn generate_sensor_data(
    cycle: u64,
    shared_feedback: Arc<Mutex<Option<FeedbackData>>>
) -> SensorArmData {
    // Use fastrand directly to generate variables below

    let object_data = if cycle % 10 == 0 {
        // Every 10th cycle, simulate an anomaly (like hand)
        generate_anomalous_object_data()
    } else {
        generate_normal_object_data()
    };

    let mut sensor_data = SensorArmData::new(object_data.clone());
    sensor_data.update_object_data(object_data);

    //realistic segment lengths (upper and lower arm)
    //l1 = shoulder to elbow (1–4cm), l2 = elbow to wrist (4–7cm)
    let l1 = 1.0 + fastrand::f32() * 3.0; // 1cm base + 0–3cm range = 1–4cm
    let l2 = 4.0 + fastrand::f32() * 3.0; // 4–7cm

    //angle from 0 (right) to π (left), but we clamp it to [0, π/2] for safe forward-right region
    let theta1 = fastrand::f32() * std::f32::consts::FRAC_PI_2; // [0, π/2]
    //elbow bend ±90°, so -π/2 to π/2 range is OK
    let theta2 = (fastrand::f32() - 0.5) * std::f32::consts::PI;

    // Lock and clone only once
    let feedback_opt = {
        let guard = shared_feedback.lock().await;
        (*guard).clone()
    };

    // Use the cloned data outside the lock
    if let Some(feedback) = feedback_opt {
        sensor_data.joints.shoulder_x = feedback.joints.shoulder_x;
        sensor_data.joints.shoulder_y = feedback.joints.shoulder_y;
        sensor_data.elbow.elbow_x = feedback.elbow.elbow_x;
        sensor_data.elbow.elbow_y = feedback.elbow.elbow_y;
        sensor_data.wrist.wrist_x = feedback.wrist.wrist_x;
        sensor_data.wrist.wrist_y = feedback.wrist.wrist_y;
    }
    else {
        //using forward kinematics to calculate arm positions
        //wrist length > elbow length > shoulder length
        //shoulder is the base of the arm, so it is the least variable
        //shoulder length can vary from 0cm to 1cm
        sensor_data.joints.shoulder_x = fastrand::f32() * 1.0;
        sensor_data.joints.shoulder_y = (fastrand::f32() * 3.0) - 1.5; // y: [-1.5, 1.5]
        //using FK to get elbow position from shoulder + angle + l1
        //this models the upper arm segment
        sensor_data.elbow.elbow_x = sensor_data.joints.shoulder_x + l1 * theta1.cos();
        sensor_data.elbow.elbow_y = sensor_data.joints.shoulder_y + l1 * theta1.sin();
        sensor_data.elbow.elbow_x = branchless_clamp(sensor_data.elbow.elbow_x, 0.0, 7.0);
        sensor_data.elbow.elbow_y = branchless_clamp(sensor_data.elbow.elbow_y, -1.5, 1.5);

        //wrist is the end of the forearm, which bends at the elbow
        //direction is based on total angle (shoulder + elbow joint)
        sensor_data.wrist.wrist_x = sensor_data.elbow.elbow_x + l2 * (theta1 + theta2).cos();
        sensor_data.wrist.wrist_y = sensor_data.elbow.elbow_y + l2 * (theta1 + theta2).sin();

        // clamp wrist_x to ≥ shoulder_x
        if sensor_data.wrist.wrist_x < sensor_data.joints.shoulder_x {
            sensor_data.wrist.wrist_x = sensor_data.joints.shoulder_x;
        }

        // clamp wrist_y to [-1.5, 1.5]
        sensor_data.wrist.wrist_y = branchless_clamp(sensor_data.wrist.wrist_y, -1.5, 1.5);
    }

    //suggested arm velocity to catch object
    sensor_data.arm_velocity = fastrand::f32() * 10.0;

    //arm strength is a crude estimate based on F = m * a,
    //assuming velocity is proportional to acceleration here
    sensor_data.arm_strength = sensor_data.arm_velocity * sensor_data.object_data.object_mass;
    sensor_data.object_data.object_height = sensor_data.joints.shoulder_y + l1 * theta1.sin() + l2 * (theta1 + theta2).sin();
    sensor_data.timestamp = now_micros();

    sensor_data
}

pub async fn publish<T>(
    channel: &lapin::Channel,
    data: &T,
) -> Result<(), Box<dyn std::error::Error>>
where
    T: Serialize,
{
    let payload = serde_json::to_vec(data)?;
    channel
        .basic_publish(
            "",
            "sensor_data",
            BasicPublishOptions::default(),
            &payload,
            Default::default(),
        )
        .await?;
        // .await
    Ok(())
}

pub fn process_sensor_data(mut raw: SensorArmData, filters: &mut Filters) -> (SensorArmData, bool) {
    // let start = now_micros();
    // let mut filtered = raw.clone();
    // destructure to reduce deep field access
    let wrist = &mut raw.wrist;
    let joints = &mut raw.joints;
    let elbow = &mut raw.elbow;
    let obj = &mut raw.object_data;

    // Apply filters
    wrist.wrist_x = filters.wrist_x_filter.update(wrist.wrist_x);
    wrist.wrist_y = filters.wrist_y_filter.update(wrist.wrist_y);
    joints.shoulder_x = filters.shoulder_x_filter.update(joints.shoulder_x);
    joints.shoulder_y = filters.shoulder_y_filter.update(joints.shoulder_y);
    elbow.elbow_x = filters.elbow_x_filter.update(elbow.elbow_x);
    elbow.elbow_y = filters.elbow_y_filter.update(elbow.elbow_y);

    raw.arm_velocity = filters.arm_velocity_filter.update(raw.arm_velocity);

    obj.object_x = filters.object_x_filter.update(obj.object_x);
    obj.object_y = filters.object_y_filter.update(obj.object_y);
    obj.object_mass = filters.object_mass_filter.update(obj.object_mass);
    obj.object_size = filters.object_size_filter.update(obj.object_size);
    obj.object_velocity = filters.object_velocity_filter.update(obj.object_velocity);
    obj.object_height = filters.object_height_filter.update(obj.object_height);

    // Recalculate derived metric
    raw.arm_strength = raw.arm_velocity * obj.object_mass;
    // // Filter joint data
    // filtered.wrist.wrist_x = filters.wrist_x_filter.update(raw.wrist.wrist_x);
    // filtered.wrist.wrist_y = filters.wrist_y_filter.update(raw.wrist.wrist_y);
    // filtered.joints.shoulder_x = filters.shoulder_x_filter.update(raw.joints.shoulder_x);
    // filtered.joints.shoulder_y = filters.shoulder_y_filter.update(raw.joints.shoulder_y);
    // filtered.elbow.elbow_x = filters.elbow_x_filter.update(raw.elbow.elbow_x);
    // filtered.elbow.elbow_y = filters.elbow_y_filter.update(raw.elbow.elbow_y);

    // // Filter arm velocity
    // filtered.arm_velocity = filters.arm_velocity_filter.update(raw.arm_velocity);

    // // Filter object data fields
    // filtered.object_data.object_x = filters.object_x_filter.update(raw.object_data.object_x);
    // filtered.object_data.object_y = filters.object_y_filter.update(raw.object_data.object_y);
    // filtered.object_data.object_mass = filters.object_mass_filter.update(raw.object_data.object_mass);
    // filtered.object_data.object_size = filters.object_size_filter.update(raw.object_data.object_size);
    // filtered.object_data.object_velocity = filters.object_velocity_filter.update(raw.object_data.object_velocity);
    // filtered.object_data.object_height = filters.object_height_filter.update(raw.object_data.object_height);

    // // Calculate arm strength after filtering
    // filtered.arm_strength = filtered.arm_velocity * filtered.object_data.object_mass;

    // Anomaly detection thresholds
    let values = [
        (raw.arm_strength, 0.0, 50.0),
        (wrist.wrist_x, 0.0, 7.0),
        (wrist.wrist_y, -7.0, 7.0),
        (joints.shoulder_x, 0.0, 7.0),
        (joints.shoulder_y, -7.0, 7.0),
        (elbow.elbow_x, 0.0, 7.0),
        (elbow.elbow_y, -7.0, 7.0),
        (obj.object_mass, 1.0, 5.0),
        (obj.object_size, 4.0, 5.0),
        (obj.object_velocity, 9.8, 11.8),
    ];

    let anomaly = values.iter().any(|(v, min, max)| detect_anomaly(*v, *min, *max));
    // let anomaly = detect_anomaly(filtered.arm_strength, 0.0, 50.0)
    //     || detect_anomaly(filtered.wrist.wrist_x, 0.0, 7.0)
    //     || detect_anomaly(filtered.wrist.wrist_y, -7.0, 7.0)
    //     || detect_anomaly(filtered.joints.shoulder_x, 0.0, 7.0)
    //     || detect_anomaly(filtered.joints.shoulder_y, -7.0, 7.0)
    //     || detect_anomaly(filtered.elbow.elbow_x, 0.0, 7.0)
    //     || detect_anomaly(filtered.elbow.elbow_y, -7.0, 7.0)
    //     || detect_anomaly(filtered.object_data.object_mass, 1.0, 5.0)             // extra mass
    //     || detect_anomaly(filtered.object_data.object_size, 4.0, 5.0)             // unusual size
    //     || detect_anomaly(filtered.object_data.object_velocity, 9.8, 11.8);       // non-moving object
    // let latency = now_micros() - start;
    // println!("Sensor data processed in {} µs", latency);
    (raw, anomaly)
}


pub async fn consume_feedback(shutdown: Arc<Notify>, shared_feedback: Arc<Mutex<Option<FeedbackData>>>, ready_notify: Arc<Notify>) {
    let conn = Connection::connect("amqp://127.0.0.1:5672/%2f", ConnectionProperties::default())
        .await.expect("Connection error");
    let channel = conn.create_channel().await.expect("Channel creation error");

    channel.queue_declare(
        "feedback_to_sensor",
        QueueDeclareOptions::default(),
        FieldTable::default(),
    ).await.expect("Queue declaration error");

    let mut consumer = channel.basic_consume(
        "feedback_to_sensor",
        "feedback_consumer",
        BasicConsumeOptions::default(),
        FieldTable::default(),
    ).await.expect("Basic consume error");

    println!("> Feedback consumer ready...");
    ready_notify.notify_waiters();
    loop {
        tokio::select! {
            maybe_delivery = consumer.next() => {
                if let Some(Ok(delivery)) = maybe_delivery {
                    let payload = &delivery.data;
                    if let Ok(feedback) = serde_json::from_slice::<FeedbackData>(payload) {
                        println!("Received feedback: {:?}", feedback);
                        //latency from feedback timestamp to now, measuring how long it took to send data and receive from controller end
                        let latency = now_micros() - feedback.timestamp;
                        println!("Reception latency: {} µs", latency);
                        let mut shared = shared_feedback.lock().await;
                        *shared = Some(feedback);
                    }
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
}