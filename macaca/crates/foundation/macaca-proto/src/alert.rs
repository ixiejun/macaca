//! Provider-neutral alert protocol DTOs.
//!
//! Alert delivery is an operating-system service contract.  These DTOs live in
//! `macaca-proto` so SDK clients, runtime-host providers, and kernel primitives
//! can exchange alert commands without importing each other's concrete modules.

use serde::{Deserialize, Serialize};

/// Provider-neutral alert severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Alert command payload shared by SDK, kernel primitives, and runtime-host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub source: String,
}

impl Alert {
    /// Build a warning alert without binding the caller to a delivery provider.
    pub fn warning(
        title: impl Into<String>,
        message: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            severity: AlertSeverity::Warning,
            title: title.into(),
            message: message.into(),
            source: source.into(),
        }
    }

    /// Build a critical alert without binding the caller to a delivery provider.
    pub fn critical(
        title: impl Into<String>,
        message: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            severity: AlertSeverity::Critical,
            title: title.into(),
            message: message.into(),
            source: source.into(),
        }
    }

    /// Build an informational alert without binding the caller to a delivery provider.
    pub fn info(
        title: impl Into<String>,
        message: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            severity: AlertSeverity::Info,
            title: title.into(),
            message: message.into(),
            source: source.into(),
        }
    }
}
