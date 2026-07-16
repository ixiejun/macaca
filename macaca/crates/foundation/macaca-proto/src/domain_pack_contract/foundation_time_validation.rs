use super::foundation_time::{
    TimeCreateTimerCommand, TimeDuration, TimeFormatSpec, TimeInstant, TimeTimerReference,
    TimeZoneReference,
};
use super::foundation_validation::{bounded_reference, opaque_artifact_reference};

impl TimeDuration {
    /// Reject invalid nanosecond adjustments before duration arithmetic or timer creation.
    pub fn is_normalized(&self) -> bool {
        (-999_999_999..=999_999_999).contains(&self.nanos_adjustment)
    }
}

impl TimeZoneReference {
    /// Keep timezone selection as a bounded identifier rather than provider payload.
    pub fn is_bounded_reference(&self) -> bool {
        bounded_reference(&self.zone_id, 96) && bounded_reference(&self.data_version, 96)
    }
}

impl TimeInstant {
    /// Validate the calendar/timezone identity that accompanies an epoch instant.
    pub fn is_bounded_reference(&self) -> bool {
        bounded_reference(&self.timezone_id, 96) && bounded_reference(&self.calendar_id, 96)
    }
}

impl TimeFormatSpec {
    /// Formatting/parsing input must be a redacted format artifact, not raw user content.
    pub fn is_safe_reference(&self) -> bool {
        opaque_artifact_reference(&self.pattern_ref)
            && bounded_reference(&self.locale.locale_id, 96)
            && self.timezone.is_bounded_reference()
    }
}

impl TimeTimerReference {
    /// Timer handles carry only bounded timer and session references.
    pub fn is_bounded_reference(&self) -> bool {
        bounded_reference(&self.timer_id, 160) && bounded_reference(&self.session_binding, 160)
    }
}

impl TimeCreateTimerCommand {
    /// Bound timer creation before the runtime reserves a timer slot.
    pub fn is_bounded_request(&self, max_duration_ms: i128) -> bool {
        self.duration.is_normalized()
            && self.duration.millis > 0
            && self.duration.millis <= max_duration_ms
            && bounded_reference(&self.session_binding, 160)
    }
}
