//! Safe dispatch outcome value object returned to Scheduler run-control.
//!
//! **Pattern:** Value Object — encodes only provider-neutral status classes that
//! Scheduler may use for retry/skip/success transitions without inspecting
//! business payloads or application-specific semantics.

/// Safe result class returned to Scheduler run-control transitions.
///
/// Scheduler interprets `succeeded`, `retryable`, and `reason_code` as
/// durable run-state signals.  No application names, prompts, or provider
/// identifiers appear in this type; those remain owned by downstream services.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomyDispatchOutcome {
    /// Whether the dispatch completed successfully from Scheduler's perspective.
    pub succeeded: bool,
    /// Whether Scheduler may schedule a bounded retry for this run.
    pub retryable: bool,
    /// Stable, provider-neutral reason token for audit and run-control logic.
    pub reason_code: &'static str,
}

impl AutonomyDispatchOutcome {
    /// Build a successful dispatch outcome.
    ///
    /// Successful outcomes are always final (`retryable = false`) because the
    /// target service accepted and completed the syscall boundary contract.
    pub fn succeeded() -> Self {
        Self {
            succeeded: true,
            retryable: false,
            reason_code: "dispatch_succeeded",
        }
    }

    /// Build a bounded failure outcome that Scheduler may retry.
    ///
    /// Retryable outcomes represent transient service failures, timeouts, or
    /// evidence gaps that may succeed on a later attempt without changing
    /// Scheduler target metadata.
    pub fn retryable(reason_code: &'static str) -> Self {
        Self {
            succeeded: false,
            retryable: true,
            reason_code,
        }
    }

    /// Build a final skipped outcome for unsupported generic categories.
    ///
    /// Skipped outcomes are non-retryable because the target category is not
    /// yet active or the payload reference could not be resolved at the OS
    /// boundary; re-dispatching without metadata changes would not help.
    pub fn skipped(reason_code: &'static str) -> Self {
        Self {
            succeeded: false,
            retryable: false,
            reason_code,
        }
    }
}
