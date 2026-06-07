//! Runtime-host payment policy boundary.
//!
//! Runtime-host providers consume payment policy through this module path to
//! avoid coupling to kernel-owned definitions while migration is in progress.

pub use macaca_proto::{DefaultPaymentPolicyEngine, PaymentPolicyDecision, PaymentPolicyEngine};
