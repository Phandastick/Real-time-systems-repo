use raw_sync::{Timeout, events::*};
use shared_memory::*;

pub fn start() -> Result<(), Box<dyn std::error::Error>> {
    println!("Actuator test starting...");

    let shmem = match ShmemConf::new()
        .size(1024)
        .flink("actuator_mapping")
        .create()
    {
        Ok(m) => m,
        Err(ShmemError::LinkExists) => ShmemConf::new().flink("actuator_mapping").open()?,
        Err(e) => return Err(Box::new(e)),
    };
    Ok(());

    if shmem.is_owner() {
        //Create an event in the shared memory
        println!("Creating event in shared memory");
        let (evt, used_bytes) = unsafe { Event::new(shmem.as_ptr(), true)? };
        println!("\tUsed {used_bytes} bytes");

        println!("Launch another instance of this example to signal the event !");
        evt.wait(Timeout::Infinite)?;
        println!("\tGot signal !");
    } else {
        // Open existing event
        println!("Openning event from shared memory");
        let (evt, used_bytes) = unsafe { Event::from_existing(shmem.as_ptr())? };
        println!("\tEvent uses {used_bytes} bytes");

        println!("Signaling event !");
        evt.set(EventState::Signaled)?;
        println!("\tSignaled !");
    };

    println!("Done !");
    Ok(())
}
