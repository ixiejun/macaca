//! Provider-neutral Application Service DTOs.
//!
//! Split from monolithic `application_service.rs` (P5 iteration 83) using a
//! Facade module tree.  These types live in `macaca-proto` because both the SDK
//! and runtime-host must exchange Application Service commands without
//! depending on `macaca-app`.  `macaca-app` still owns application semantics;
//! this module is only the wire-safe command/result vocabulary used by service
//! clients, providers, Web adapters, and future remote transports.
//!
//! # Design patterns
//! - **Facade**: `mod.rs` re-exports preserve `macaca_proto::application_service::*`.
//! - **Command**: typed lifecycle/query/delegate command DTOs map to service envelopes.
//! - **DTO / View**: sanitized read models for Web, CLI, SDK, and audit consumers.
//! - **Memento**: `ApplicationServiceSnapshot` captures replay-safe service state.
//! - **Adapter**: `into_service_command` bridges typed commands to `ServiceCommand`.

mod app_views;
mod constants;
mod delegate;
mod lifecycle_commands;
mod metadata_views;
mod query_commands;
mod scope;
mod snapshot;
mod ui_bridge;
mod validation;

pub use app_views::*;
pub use constants::*;
pub use delegate::*;
pub use lifecycle_commands::*;
pub use metadata_views::*;
pub use query_commands::*;
pub use scope::*;
pub use snapshot::*;
pub use ui_bridge::*;
