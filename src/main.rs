use std::thread::spawn;

mod actuator_module;
mod data_structure;
mod sensor_module;

fn main() {
    let _ = spawn(|| {
        actuator_module::main::actuator_start();
    });

    let _ = spawn(|| {
        sensor_module::main::sensor_start();
    })
    .join();
}
