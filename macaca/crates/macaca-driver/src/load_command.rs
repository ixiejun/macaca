//! Driver runtime load command and report types.

use std::path::PathBuf;

/// Driver runtime load intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverLoadCommand {
    LoadAll,
    Reload,
}

/// Aggregate report for a driver load command.
#[derive(Debug, Clone)]
pub struct DriverLoadReport {
    pub command: DriverLoadCommand,
    pub loaded: usize,
    pub failed: usize,
    pub entries: Vec<DriverLoadEntry>,
}

/// Per-driver load report entry.
#[derive(Debug, Clone)]
pub struct DriverLoadEntry {
    pub name: String,
    pub path: PathBuf,
    pub status: DriverLoadStatus,
    pub tool_count: Option<usize>,
    pub error: Option<String>,
}

/// Per-driver load status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverLoadStatus {
    Loaded,
    Failed,
}
