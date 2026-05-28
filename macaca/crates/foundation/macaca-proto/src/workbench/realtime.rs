//! Optional realtime service contracts.
//!
//! `service.realtime` is an optional Adapter boundary for realtime text,
//! audio, and transport sessions.  The base OS can run without any realtime
//! provider; absent providers return structured unavailable results.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{BoundedSummary, WorkbenchCommand, WorkbenchServiceDescriptor};

pub const SERVICE_ID: &str = "service.realtime";
pub const CAPABILITY_ID: &str = "capability.realtime";
pub const START_COMMAND: &str = "realtime.start";
pub const APPEND_TEXT_COMMAND: &str = "realtime.append_text";
pub const APPEND_AUDIO_COMMAND: &str = "realtime.append_audio";
pub const STOP_COMMAND: &str = "realtime.stop";
pub const HEALTH_COMMAND: &str = "realtime.health";

pub const COMMANDS: &[&str] = &[
    START_COMMAND,
    APPEND_TEXT_COMMAND,
    APPEND_AUDIO_COMMAND,
    STOP_COMMAND,
    HEALTH_COMMAND,
];

pub type RealtimeCommand<T> = WorkbenchCommand<T>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RealtimeModality {
    Text,
    Audio,
    TextAndAudio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RealtimeTransportKind {
    WebRtc,
    WebSocket,
    LocalLoopback,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RealtimeSessionStatus {
    Starting,
    Active,
    Stopped,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealtimeStartRequest {
    pub modality: RealtimeModality,
    pub transport: RealtimeTransportKind,
    pub provider_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealtimeTextAppendRequest {
    pub session_ref: String,
    pub text: BoundedSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealtimeAudioAppendRequest {
    pub session_ref: String,
    pub audio_ref: super::ArtifactRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealtimeSessionRefRequest {
    pub session_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealtimeSessionRecord {
    pub session_ref: String,
    pub modality: RealtimeModality,
    pub transport: RealtimeTransportKind,
    pub status: RealtimeSessionStatus,
    pub connection_state: BoundedSummary,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RealtimeServiceResponse {
    Session(RealtimeSessionRecord),
    Accepted {
        session_ref: String,
    },
    Health {
        configured: bool,
        reason: Option<String>,
    },
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.realtime.events.v1",
        "service.realtime.audit.v1",
        "service.realtime.snapshot.v1",
    )
}
