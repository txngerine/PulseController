pub mod error;
pub mod collector;

pub use error::{TelemetryError, Result};
pub use collector::{TelemetryCollector, MetricsSnapshot};
