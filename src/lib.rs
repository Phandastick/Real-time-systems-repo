pub mod data_structure;

// lib.rs or a shared module
#[derive(Clone, Debug)]
pub struct Command {
    pub id: u64,
    pub timestamp: std::time::Instant, // to measure latency
}
