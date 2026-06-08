//! Stable Application Service identifiers and command vocabulary.
//!
//! Command name constants are the wire contract between SDK clients, runtime-host
//! providers, and shell adapters.  Keeping them centralized prevents drift across
//! registration, routing, and audit log correlation.

/// Stable service id used by Application Service registration and SDK clients.
pub const APPLICATION_SERVICE_ID: &str = "service.application";

/// Command names accepted by the Application Service provider adapter.
pub const APPLICATION_DISCOVER_COMMAND: &str = "application.discover";
pub const APPLICATION_LOAD_COMMAND: &str = "application.load";
pub const APPLICATION_START_COMMAND: &str = "application.start";
pub const APPLICATION_STOP_COMMAND: &str = "application.stop";
pub const APPLICATION_REMOVE_COMMAND: &str = "application.remove";
pub const APPLICATION_STATUS_COMMAND: &str = "application.status";
pub const APPLICATION_SNAPSHOT_COMMAND: &str = "application.snapshot";
pub const APPLICATION_SESSION_START_COMMAND: &str = "application.session.start";
pub const APPLICATION_SESSION_RESUME_COMMAND: &str = "application.session.resume";
pub const APPLICATION_SESSION_STOP_COMMAND: &str = "application.session.stop";
pub const APPLICATION_HOST_DISPATCH_COMMAND: &str = "application.host.dispatch";
pub const APPLICATION_GENUI_SURFACE_COMMAND: &str = "application.genui.surface";
pub const APPLICATION_METADATA_QUERY_COMMAND: &str = "application.metadata.query";
pub const APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND: &str = "application.heartbeat.agents.query";
pub const APPLICATION_AGENT_DELEGATE_COMMAND: &str = "application.agent.delegate";
