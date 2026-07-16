pub mod error;
pub mod traits;
pub mod windows;
pub mod macos;

pub use error::{PlatformError, Result};
pub use traits::{InputBackend, BackendConfig, BackendEvent};
