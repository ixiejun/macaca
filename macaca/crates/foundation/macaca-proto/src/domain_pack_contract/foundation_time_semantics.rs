//! Admission and resource semantics for the foundation time pack.
//!
//! This module contains pure specifications.  It deliberately does not read a
//! clock or create a timer, which lets the kernel and application framework
//! reject unsafe requests before a concrete provider can cause side effects.

use serde::{Deserialize, Serialize};

use super::foundation_time::{TimeCreateTimerCommand, TimeExactnessHint};

/// Policy facts supplied by the composition root for one time request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeAdmissionContext {
    pub declared_scopes: Vec<String>,
    pub timer_count: u32,
    pub mock_clock_context: bool,
    pub provider_available: bool,
    pub supports_exact_timers: bool,
    pub supports_mock_clock: bool,
    pub max_timer_duration_ms: i128,
    pub max_active_timers: u32,
}

/// Resource limits used by timer admission and lifecycle release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeResourceLimits {
    pub max_active_timers: u32,
    pub max_timer_duration_ms: i128,
}

/// Opaque reservation returned before timer creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeResourceReservation {
    pub reservation_id: String,
    pub timer_count: u32,
    pub duration_ms: i128,
}

/// Structured fail-closed admission outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeAdmissionFailure {
    MissingScope(String),
    ProviderUnavailable,
    QuotaExceeded,
    DurationExceeded,
    ExactTimerUnsupported,
    MockClockDenied,
    MockClockUnsupported,
    InvalidRequest(String),
}

/// Check timer policy without invoking a provider or reserving host resources.
pub fn preflight_timer(
    command: &TimeCreateTimerCommand,
    context: &TimeAdmissionContext,
) -> Result<TimeResourceReservation, TimeAdmissionFailure> {
    require_scope(context, "time.timer")?;
    if !context.provider_available {
        return Err(TimeAdmissionFailure::ProviderUnavailable);
    }
    if context.timer_count >= context.max_active_timers {
        return Err(TimeAdmissionFailure::QuotaExceeded);
    }
    if !command.is_bounded_request(context.max_timer_duration_ms) {
        return Err(TimeAdmissionFailure::DurationExceeded);
    }
    if matches!(command.exactness, TimeExactnessHint::ExactRequired)
        && !context.supports_exact_timers
    {
        return Err(TimeAdmissionFailure::ExactTimerUnsupported);
    }
    if context.mock_clock_context && !context.supports_mock_clock {
        return Err(TimeAdmissionFailure::MockClockUnsupported);
    }
    Ok(TimeResourceReservation {
        reservation_id: format!("time-reservation-{}", context.timer_count + 1),
        timer_count: context.timer_count + 1,
        duration_ms: command.duration.millis,
    })
}

/// Reserve one timer slot using the same rules as preflight.
pub fn reserve_timer(
    command: &TimeCreateTimerCommand,
    context: &TimeAdmissionContext,
) -> Result<TimeResourceReservation, TimeAdmissionFailure> {
    preflight_timer(command, context)
}

/// Release is represented as a pure transition so every terminal path can be audited.
pub fn release_timer(reservation: &TimeResourceReservation) -> TimeResourceReservation {
    TimeResourceReservation {
        reservation_id: format!("released:{}", reservation.reservation_id),
        timer_count: reservation.timer_count.saturating_sub(1),
        duration_ms: 0,
    }
}

fn require_scope(context: &TimeAdmissionContext, scope: &str) -> Result<(), TimeAdmissionFailure> {
    if context.declared_scopes.iter().any(|value| value == scope) {
        Ok(())
    } else {
        Err(TimeAdmissionFailure::MissingScope(scope.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_pack_contract::{TimeDuration, TimeExactnessHint};

    fn command(duration: i128, exactness: TimeExactnessHint) -> TimeCreateTimerCommand {
        TimeCreateTimerCommand {
            duration: TimeDuration {
                millis: duration,
                nanos_adjustment: 0,
            },
            exactness,
            session_binding: "session:test".into(),
        }
    }

    fn context() -> TimeAdmissionContext {
        TimeAdmissionContext {
            declared_scopes: vec!["time.timer".into()],
            timer_count: 0,
            mock_clock_context: false,
            provider_available: true,
            supports_exact_timers: true,
            supports_mock_clock: false,
            max_timer_duration_ms: 1_000,
            max_active_timers: 2,
        }
    }

    #[test]
    fn rejects_policy_failures_before_provider_invocation() {
        assert_eq!(
            preflight_timer(
                &command(10, TimeExactnessHint::ExactPreferred),
                &TimeAdmissionContext {
                    declared_scopes: vec![],
                    ..context()
                }
            ),
            Err(TimeAdmissionFailure::MissingScope("time.timer".into()))
        );
        assert_eq!(
            preflight_timer(
                &command(2_000, TimeExactnessHint::InexactAllowed),
                &context()
            ),
            Err(TimeAdmissionFailure::DurationExceeded)
        );
        assert_eq!(
            preflight_timer(
                &command(10, TimeExactnessHint::ExactRequired),
                &TimeAdmissionContext {
                    supports_exact_timers: false,
                    ..context()
                }
            ),
            Err(TimeAdmissionFailure::ExactTimerUnsupported)
        );
    }

    #[test]
    fn reservation_release_is_bounded_and_auditable() {
        let reservation =
            reserve_timer(&command(10, TimeExactnessHint::InexactAllowed), &context()).unwrap();
        let released = release_timer(&reservation);
        assert!(released.reservation_id.starts_with("released:"));
        assert_eq!(released.duration_ms, 0);
    }
}
