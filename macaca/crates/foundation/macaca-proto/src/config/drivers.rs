//! External driver plugin discovery configuration.

use serde::{Deserialize, Serialize};

/// External driver plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriversConfig {
    /// Directory containing external driver plugins (relative to working dir or absolute).
    #[serde(default = "default_drivers_directory")]
    pub directory: String,
    /// Whether to automatically load all drivers at startup.
    #[serde(default = "default_auto_load")]
    pub auto_load: bool,
}

fn default_drivers_directory() -> String {
    "drivers".to_string()
}

fn default_auto_load() -> bool {
    true
}

impl Default for DriversConfig {
    fn default() -> Self {
        Self {
            directory: default_drivers_directory(),
            auto_load: default_auto_load(),
        }
    }
}
