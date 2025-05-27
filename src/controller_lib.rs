use crate::data_structure::*;
use rand::random;

// PUT CONTROLLER LOGIC TO BE CALLED IN LIB.rs
// TRY TO NOT CLUTTER THE lib.rs FILE PLEASEEEEE

pub fn generate_sensor_data(cycle: u64) -> SensorArmData {
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
            object_x: random::<f32>() * 3.0,
            //-1.5 to 1.5
            object_y: (random::<f32>() * 3.0) - 1.5,
            //object height is the distance from the top of the tube to the object
            //calculated later based on arm
            object_height: 0.0,
        }
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
    sensor_data.object_data.object_height =
        sensor_data.joints.shoulder_y + l1 * theta1.sin() + l2 * (theta1 + theta2).sin();
    sensor_data.timestamp = now_micros();

    sensor_data
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
        object_x: 2.5 + random::<f32>() * 3.0, // 2.5–5.5 cm (outside normal tube center)

        // -1.5 to 1.5, but hand might extend further
        object_y: -2.0 + random::<f32>() * 4.0, // -2.0 to +2.0 (possibly out of vertical bounds)

        object_height: 0.0,
    }
}
