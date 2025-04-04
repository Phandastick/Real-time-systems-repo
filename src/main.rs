mod actuator_module;
mod data_structure;
mod sensor_module;

fn main() {
    sensor_module::main::sensor_start();
    actuator_module::main::actuator_start();
    println!("helo");
}
