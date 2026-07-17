pub mod error;
pub mod traits;
pub mod windows;
pub mod macos;
#[cfg(target_os = "linux")]
pub mod linux;

pub use error::{PlatformError, Result};
pub use traits::{InputBackend, BackendConfig, BackendEvent};
