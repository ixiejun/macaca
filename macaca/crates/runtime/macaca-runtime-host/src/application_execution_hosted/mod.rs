//! Macaca-hosted application execution provider strategy (Facade module tree).
//!
//! This module is the runtime-host Adapter for the `MacacaHosted` provider kind.
//! It does not contain application-specific behavior. The provider owns the generic
//! execution envelope, durable event emission, and control routing, while concrete
//! application logic enters through an injected [`HostedApplicationExecutionAdapter`].
//!
//! Module layout:
//! - `types.rs` — protocol-neutral outcomes, signals, and Strategy trait seams
//! - `abi_adapter.rs` — Application ABI bridge (Adapter pattern)
//! - `host_signal_translator.rs` — Observer bridge from host-command rows to signals
//! - `provider.rs` — MacacaHosted provider construction and EventLog helpers
//! - `provider_impl.rs` — ApplicationExecutionProvider trait state machine

mod abi_adapter;
mod host_signal_translator;
mod provider;
mod provider_impl;
mod types;

pub use abi_adapter::ApplicationAbiHostedExecutionAdapter;
pub use provider::MacacaHostedApplicationExecutionProvider;
pub use types::{HostedApplicationExecutionAdapter, HostedApplicationExecutionOutcome};
