pub mod error;
pub mod manager;

pub use error::{DiscoveryError, Result};
pub use manager::{DiscoveryManager, DiscoveredDevice};
