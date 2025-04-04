mod actuator_module;
mod data_structure;
mod sensor_module;

#[tokio::main]
async fn main() {
    let _promise = sensor_module::main::sensor_start();
    let _promise2 = actuator_module::main::actuator_start();
}
