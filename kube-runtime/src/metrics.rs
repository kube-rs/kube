//! Optional metrics exposed by the runtime
use parking_lot::RwLock;
use std::sync::Arc;

/// Metrics relating to the `Scheduler`
#[derive(Default, Debug)]
pub struct SchedulerMetrics {
    /// Current size of the scheduler queue
    pub queue_depth: usize,
}

/// All metrics
#[derive(Default, Debug)]
pub struct Metrics {
    /// kube build info
    pub build_info: String,
    /// Metrics from the scheduler
    pub scheduler: Arc<RwLock<SchedulerMetrics>>,
}

impl Metrics {
    fn new() -> Self {
        Self {
            build_info: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        }
    }
}
