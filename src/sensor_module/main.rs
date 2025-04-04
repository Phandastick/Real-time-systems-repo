use crate::sensor_module::sensor;

pub async fn sensor_start(){
    print!("Sensor module started\n");
    sensor::force_sensor();
}