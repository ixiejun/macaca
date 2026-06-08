//! Strongly-typed identifier newtypes for cross-crate domain references.
//!
//! Each ID wraps a `Uuid` with serde support so wire formats stay stable while
//! compile-time distinctions prevent accidental mixing of unrelated identifiers.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Identity Types ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a forked (child) agent instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ForkId(pub Uuid);

impl ForkId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ForkId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ForkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fork-{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub Uuid);

impl MemoryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApplicationId(pub Uuid);

impl ApplicationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a deterministic ApplicationId from an app name.
    /// Same name always produces the same ID across restarts.
    pub fn from_name(name: &str) -> Self {
        // UUID v5 with a fixed namespace ensures deterministic IDs
        const MACACA_NS: Uuid = Uuid::from_bytes([
            0x6d, 0x61, 0x63, 0x61, 0x63, 0x61, 0x2d, 0x6f, 0x73, 0x2d, 0x61, 0x70, 0x70, 0x2d,
            0x6e, 0x73,
        ]);
        Self(Uuid::new_v5(&MACACA_NS, name.as_bytes()))
    }
}

impl Default for ApplicationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ApplicationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DriverId(pub Uuid);

impl DriverId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DriverId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DriverId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
