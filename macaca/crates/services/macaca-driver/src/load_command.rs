//! Driver runtime load protocol re-export.
//!
//! Load reports are serialized across the Driver service boundary, so their
//! canonical definitions live in `macaca-proto`.  This module keeps the
//! provider crate's internal path stable while avoiding duplicate DTOs.

pub use macaca_proto::{DriverLoadCommand, DriverLoadEntry, DriverLoadReport, DriverLoadStatus};
