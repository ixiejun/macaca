use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::communication_common::{
    communication_pack_definition, communication_stable_hash, schema_set,
    CommunicationPackDescriptor, CommunicationProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

/// Stable pack id for provider-neutral calendar operations.
pub const COMMUNICATION_CALENDAR_PACK_ID: &str = "pack.communication.calendar.v1";
/// Stable service id used by future calendar providers.
pub const COMMUNICATION_CALENDAR_SERVICE_ID: &str = "service.communication.calendar";

/// Canonical command names described by `pack.communication.calendar.v1`.
pub const COMMUNICATION_CALENDAR_COMMANDS: &[&str] = &[
    "calendar.list_calendars",
    "calendar.query_events",
    "calendar.get_event",
    "calendar.create_event",
    "calendar.update_event",
    "calendar.delete_event",
    "calendar.respond_invite",
    "calendar.check_availability",
    "calendar.propose_times",
    "calendar.set_reminder",
    "calendar.manage_conference",
    "calendar.import_icalendar",
    "calendar.export_icalendar",
    "calendar.register_watch",
    "calendar.sync_events",
    "calendar.inspect_conflicts",
];

const CALENDAR_PERMISSION_SCOPES: &[&str] = &[
    "calendar.read.metadata",
    "calendar.read.details",
    "calendar.write",
    "calendar.invite.send",
    "calendar.invite.respond",
    "calendar.availability",
    "calendar.reminder",
    "calendar.conference",
    "calendar.sync",
    "calendar.watch",
    "calendar.import_export",
];

const CALENDAR_SYNC_METADATA: &[(&str, &str)] = &[
    ("event_crud", "true"),
    ("availability", "true"),
    ("sync_watch", "true"),
    ("icalendar", "true"),
];
const AVAILABILITY_BRIDGE_METADATA: &[(&str, &str)] = &[
    ("event_crud", "false"),
    ("availability", "true"),
    ("sync_watch", "false"),
    ("icalendar", "false"),
];
const EVENT_STORE_METADATA: &[(&str, &str)] = &[
    ("event_crud", "true"),
    ("availability", "false"),
    ("sync_watch", "true"),
    ("icalendar", "true"),
];
const CALENDAR_MOCK_METADATA: &[(&str, &str)] = &[
    ("event_crud", "true"),
    ("availability", "true"),
    ("sync_watch", "true"),
    ("icalendar", "true"),
];
const CALENDAR_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("event_crud", "false"),
    ("availability", "false"),
    ("sync_watch", "false"),
    ("icalendar", "false"),
];

const CALENDAR_PROVIDER_CLASSES: &[CommunicationProviderClass<'_>] = &[
    CommunicationProviderClass {
        provider_class: "calendar-sync",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CALENDAR_SYNC_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "availability-bridge",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: AVAILABILITY_BRIDGE_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "event-store",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: EVENT_STORE_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CALENDAR_MOCK_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: CALENDAR_UNAVAILABLE_METADATA,
    },
];

/// Build the descriptor-only catalog entry for `pack.communication.calendar.v1`.
pub fn communication_calendar_pack_definition() -> DomainPackDefinition {
    communication_pack_definition(CommunicationPackDescriptor {
        slug: "calendar",
        service_id: COMMUNICATION_CALENDAR_SERVICE_ID,
        commands: COMMUNICATION_CALENDAR_COMMANDS,
        permission_scopes: CALENDAR_PERMISSION_SCOPES,
        provider_classes: CALENDAR_PROVIDER_CLASSES,
        health_probe: "calendar.list_calendars",
        unavailable_reason: "calendar_provider_not_installed",
        replay_schema: "calendar.pack.replay.v1",
        data_classification: "calendar_event_metadata",
        retention_policy: "event_metadata_and_handles_with_redacted_descriptions",
        redaction_policy: "credentials_invite_payloads_calendar_exports_and_conference_secrets_redacted",
        examples: &[
            "Declare `pack.communication.calendar.v1` as optional until a calendar provider is installed.",
            "Use event handles, recurrence metadata, and iCalendar references instead of raw provider exports.",
        ],
        migration_notes: &[
            "Calendar becomes callable only after an approved communication provider registers command schemas.",
            "Provider-native event, invite, watch, and conference payloads must remain behind provider adapters.",
        ],
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarSource {
    pub source_id: String,
    pub display_name: String,
    pub owner_hash: String,
    pub timezone_id: String,
    pub provider_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub event_id: String,
    pub source_id: String,
    pub title_ref: String,
    pub description_ref: Option<String>,
    pub start_epoch_ms: i128,
    pub end_epoch_ms: i128,
    pub timezone_id: String,
    pub recurrence: Option<CalendarRecurrence>,
    pub attendees: Vec<CalendarAttendee>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarInstance {
    pub event_id: String,
    pub instance_id: String,
    pub original_start_epoch_ms: i128,
    pub start_epoch_ms: i128,
    pub end_epoch_ms: i128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarRecurrence {
    pub frequency: String,
    pub interval: u32,
    pub count: Option<u32>,
    pub until_epoch_ms: Option<i128>,
    pub timezone_id: String,
    pub expansion_limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarAttendee {
    pub attendee_id: String,
    pub role: String,
    pub response_state: String,
    pub identity_scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarAvailabilityQuery {
    pub participant_ids: Vec<String>,
    pub start_epoch_ms: i128,
    pub end_epoch_ms: i128,
    pub timezone_id: String,
    pub granularity_minutes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarReminder {
    pub reminder_id: String,
    pub offset_minutes: i32,
    pub channel_handle: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarConference {
    pub conference_id: String,
    pub join_url_ref: Option<String>,
    pub secret_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarCursor {
    pub source_id: String,
    pub cursor_hash: String,
    pub watermark_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarWatch {
    pub watch_id: String,
    pub source_id: String,
    pub callback_ref: String,
    pub expires_epoch_ms: Option<i128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarConflict {
    pub event_id: String,
    pub conflict_version: String,
    pub reason_code: String,
    pub replay_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    pub supports_event_crud: bool,
    pub supports_recurrence: bool,
    pub supports_availability: bool,
    pub supports_sync_watch: bool,
    pub supports_icalendar: bool,
    pub max_recurrence_expansion: u32,
    pub availability: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarProviderSnapshot {
    pub descriptor_hash: String,
    pub provider_class: String,
    pub source_count: u32,
    pub watch_count: u32,
    pub cursor_hashes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarListCalendarsCommand {
    pub page_size: u32,
    pub cursor_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarQueryEventsCommand {
    pub source: CalendarSource,
    pub start_epoch_ms: i128,
    pub end_epoch_ms: i128,
    pub expansion_limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarGetEventCommand {
    pub event_id: String,
    pub projection: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarCreateEventCommand {
    pub event: CalendarEvent,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarUpdateEventCommand {
    pub event: CalendarEvent,
    pub conflict_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarDeleteEventCommand {
    pub event_id: String,
    pub cancel_with_notice: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarRespondInviteCommand {
    pub event_id: String,
    pub attendee_id: String,
    pub response_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarCheckAvailabilityCommand {
    pub query: CalendarAvailabilityQuery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarProposeTimesCommand {
    pub query: CalendarAvailabilityQuery,
    pub max_candidates: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarSetReminderCommand {
    pub event_id: String,
    pub reminders: Vec<CalendarReminder>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarManageConferenceCommand {
    pub event_id: String,
    pub conference: Option<CalendarConference>,
    pub operation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarImportIcalendarCommand {
    pub source: CalendarSource,
    pub content_ref: String,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarExportIcalendarCommand {
    pub event_ids: Vec<String>,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarRegisterWatchCommand {
    pub source: CalendarSource,
    pub watch: CalendarWatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarSyncEventsCommand {
    pub source: CalendarSource,
    pub cursor: Option<CalendarCursor>,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarInspectConflictsCommand {
    pub event_id: String,
    pub conflict_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarResultStatus {
    Success,
    Page,
    PartialSync,
    ResetRequired,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    Timeout,
    Canceled,
    ValidationFailed,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarError {
    pub code: CalendarResultStatus,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarResultEnvelope<T> {
    pub status: CalendarResultStatus,
    pub data: Option<T>,
    pub error: Option<CalendarError>,
    pub trace_id: String,
    pub descriptor_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub snapshot_schema_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn communication_calendar_descriptor_hashes() -> CalendarDescriptorHashes {
    CalendarDescriptorHashes {
        command_schema_hash: calendar_stable_hash(&COMMUNICATION_CALENDAR_COMMANDS),
        result_schema_hash: calendar_stable_hash(&CalendarResultStatus::Success),
        snapshot_schema_hash: calendar_stable_hash(&CalendarProviderSnapshot {
            descriptor_hash: "descriptor".into(),
            provider_class: "unavailable".into(),
            source_count: 0,
            watch_count: 0,
            cursor_hashes: BTreeMap::new(),
        }),
        provider_capability_schema_hash: calendar_stable_hash(&CalendarProviderCapability {
            provider_class: "unavailable".into(),
            supported_commands: schema_set(COMMUNICATION_CALENDAR_COMMANDS),
            supports_event_crud: false,
            supports_recurrence: false,
            supports_availability: false,
            supports_sync_watch: false,
            supports_icalendar: false,
            max_recurrence_expansion: 0,
            availability: DomainPackProviderCapabilityState::Unavailable,
        }),
        unavailable_schema_hash: calendar_stable_hash(&CalendarError {
            code: CalendarResultStatus::Unavailable,
            message: "calendar provider is not installed".into(),
            retryable: false,
        }),
    }
}

pub fn calendar_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    communication_stable_hash(value)
}
