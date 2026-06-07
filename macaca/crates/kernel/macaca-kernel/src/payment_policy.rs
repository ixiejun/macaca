//! Compatibility re-export for payment policy types.
//!
//! The canonical policy implementation now lives in `macaca-proto` so kernel
//! and runtime-host can share one provider-neutral definition.

pub use macaca_proto::{DefaultPaymentPolicyEngine, PaymentPolicyDecision, PaymentPolicyEngine};
