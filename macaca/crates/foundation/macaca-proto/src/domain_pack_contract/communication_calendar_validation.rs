use super::communication_calendar::{
    CalendarAttendee, CalendarAvailabilityQuery, CalendarConference, CalendarCreateEventCommand,
    CalendarCursor, CalendarEvent, CalendarExportIcalendarCommand, CalendarImportIcalendarCommand,
    CalendarRecurrence, CalendarReminder, CalendarSource, CalendarWatch,
};
use super::communication_common::{bounded_communication_token, optional_secret_reference_is_safe};

impl CalendarSource {
    /// Validate calendar source metadata without exposing credentials or provider payloads.
    pub fn has_safe_identity(&self) -> bool {
        bounded_communication_token(&self.source_id, 160)
            && bounded_communication_token(&self.display_name, 160)
            && bounded_communication_token(&self.owner_hash, 256)
            && self.timezone_id.contains('/')
            && bounded_communication_token(&self.timezone_id, 96)
            && bounded_communication_token(&self.provider_class, 96)
    }
}

impl CalendarEvent {
    /// Validate write-side event metadata before provider dispatch.
    pub fn has_write_preconditions(&self, max_attendees: usize, max_recurrence: u32) -> bool {
        bounded_communication_token(&self.event_id, 160)
            && bounded_communication_token(&self.source_id, 160)
            && bounded_communication_token(&self.title_ref, 256)
            && self
                .description_ref
                .as_deref()
                .is_none_or(|reference| bounded_communication_token(reference, 256))
            && self.start_epoch_ms < self.end_epoch_ms
            && bounded_communication_token(&self.timezone_id, 96)
            && self.attendees.len() <= max_attendees
            && self
                .attendees
                .iter()
                .all(CalendarAttendee::is_safe_reference)
            && self
                .recurrence
                .as_ref()
                .is_none_or(|recurrence| recurrence.is_within_limit(max_recurrence))
    }
}

impl CalendarRecurrence {
    /// Validate recurrence expansion bounds and timezone metadata.
    pub fn is_within_limit(&self, max_expansion: u32) -> bool {
        matches!(
            self.frequency.as_str(),
            "daily" | "weekly" | "monthly" | "yearly"
        ) && self.interval > 0
            && self.expansion_limit > 0
            && self.expansion_limit <= max_expansion
            && self.count.is_none_or(|count| count <= max_expansion)
            && bounded_communication_token(&self.timezone_id, 96)
    }
}

impl CalendarAttendee {
    /// Validate invite participants as redacted identity handles.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.attendee_id, 160)
            && matches!(self.role.as_str(), "required" | "optional" | "resource")
            && matches!(
                self.response_state.as_str(),
                "needs_action" | "accepted" | "declined" | "tentative"
            )
            && bounded_communication_token(&self.identity_scope, 96)
    }
}

impl CalendarAvailabilityQuery {
    /// Validate free/busy queries without exposing private event details.
    pub fn is_bounded(&self, max_participants: usize) -> bool {
        !self.participant_ids.is_empty()
            && self.participant_ids.len() <= max_participants
            && self
                .participant_ids
                .iter()
                .all(|participant| bounded_communication_token(participant, 160))
            && self.start_epoch_ms < self.end_epoch_ms
            && bounded_communication_token(&self.timezone_id, 96)
            && matches!(self.granularity_minutes, 5 | 10 | 15 | 30 | 60)
    }
}

impl CalendarReminder {
    /// Validate reminder metadata as bounded handles.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.reminder_id, 160)
            && self.offset_minutes.abs() <= 60 * 24 * 30
            && bounded_communication_token(&self.channel_handle, 160)
    }
}

impl CalendarConference {
    /// Validate conference metadata without raw join URLs or passcodes.
    pub fn is_handle_only(&self) -> bool {
        bounded_communication_token(&self.conference_id, 160)
            && self
                .join_url_ref
                .as_deref()
                .is_none_or(|reference| bounded_communication_token(reference, 256))
            && optional_secret_reference_is_safe(self.secret_ref.as_deref())
    }
}

impl CalendarCursor {
    /// Validate sync cursors as hashes and watermarks only.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.source_id, 160)
            && bounded_communication_token(&self.cursor_hash, 256)
            && self
                .watermark_hash
                .as_deref()
                .is_none_or(|hash| bounded_communication_token(hash, 256))
    }
}

impl CalendarWatch {
    /// Validate watch callbacks as registered callback handles, never raw webhook URLs.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.watch_id, 160)
            && bounded_communication_token(&self.source_id, 160)
            && bounded_communication_token(&self.callback_ref, 256)
            && self.expires_epoch_ms.is_none_or(|expiry| expiry > 0)
    }
}

impl CalendarCreateEventCommand {
    /// Validate idempotent event creation before provider calls.
    pub fn has_admission_preconditions(&self, max_attendees: usize, max_recurrence: u32) -> bool {
        self.event
            .has_write_preconditions(max_attendees, max_recurrence)
            && bounded_communication_token(&self.idempotency_key, 128)
    }
}

impl CalendarImportIcalendarCommand {
    /// Validate import requests use content references, not raw iCalendar content.
    pub fn is_reference_only(&self) -> bool {
        self.source.has_safe_identity() && bounded_communication_token(&self.content_ref, 256)
    }
}

impl CalendarExportIcalendarCommand {
    /// Validate export requests stay bounded and redacted.
    pub fn is_bounded_export(&self, max_events: usize) -> bool {
        !self.event_ids.is_empty()
            && self.event_ids.len() <= max_events
            && self
                .event_ids
                .iter()
                .all(|event_id| bounded_communication_token(event_id, 160))
            && bounded_communication_token(&self.redaction_profile, 160)
    }
}
