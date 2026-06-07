//! CLI transport types for Skill governance commands.
//!
//! These value objects carry operator evidence and optional live-runtime targeting.
//! They contain no business policy: lifecycle semantics remain owned by
//! `service.skill` on the server side.

use macaca_proto::{MacacaError, MacacaResult};

use super::live_client::LiveSkillOperationsClient;

/// Shared operator evidence refs accepted by mutating CLI commands.
///
/// Each field is optional because diagnostic invocations may run without full
/// approval metadata; the server enforces policy when a live runtime is targeted.
#[derive(Debug, Clone, Default)]
pub struct SkillCliEvidenceRefs {
    pub reason: Option<String>,
    pub evidence_ref: Option<String>,
    pub policy_ref: Option<String>,
    pub approval_ref: Option<String>,
}

/// Optional live runtime target for CLI Skill commands.
///
/// When an application id is supplied, CLI reaches the already-running Web shell
/// through its public HTTP facade so every command observes the same live
/// `service.skill` runtime that the frontend and API use.  When no app id is
/// supplied, commands deliberately keep the SDK Null Object path as a diagnostic
/// unavailable state instead of pretending a live runtime exists.
#[derive(Debug, Clone, Default)]
pub struct SkillCliRuntimeTarget {
    pub app_id: Option<String>,
    pub api_base: Option<String>,
}

impl SkillCliRuntimeTarget {
    /// Resolve an optional live HTTP adapter when a valid application id is present.
    ///
    /// Returns `Ok(None)` when no app id was supplied (Null Object path).
    /// Returns `Err` when the id is non-empty but not a valid UUID or when the
    /// API base URL fails validation.
    pub(crate) fn live_client(&self) -> MacacaResult<Option<LiveSkillOperationsClient>> {
        let Some(app_id) = self
            .app_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(None);
        };
        uuid::Uuid::parse_str(app_id).map_err(|error| {
            MacacaError::Config(format!(
                "skill live operations require a valid app id: {error}"
            ))
        })?;
        let api_base = self
            .api_base
            .clone()
            .or_else(|| std::env::var("MACACA_API_BASE").ok())
            .unwrap_or_else(|| "http://127.0.0.1:3001".into());
        Ok(Some(LiveSkillOperationsClient::new(
            api_base,
            app_id.into(),
        )?))
    }
}

/// Lifecycle actions exposed by the CLI transport.
///
/// The enum is a **Strategy** mapping layer: CLI verbs are translated once into
/// SDK `SkillCurationLifecycleAction` values and HTTP route segments.
#[derive(Debug, Clone, Copy)]
pub enum SkillCliLifecycleAction {
    Pin,
    Unpin,
    Archive,
    Restore,
    Quarantine,
    ReleaseQuarantine,
    Reject,
}

impl SkillCliLifecycleAction {
    /// Map the CLI verb to the SDK lifecycle action enum consumed by `service.skill`.
    pub(crate) fn into_service_action(self) -> macaca_sdk::SkillCurationLifecycleAction {
        use macaca_sdk::SkillCurationLifecycleAction;
        match self {
            Self::Pin => SkillCurationLifecycleAction::Pin,
            Self::Unpin => SkillCurationLifecycleAction::Unpin,
            Self::Archive => SkillCurationLifecycleAction::Archive,
            Self::Restore => SkillCurationLifecycleAction::Restore,
            Self::Quarantine => SkillCurationLifecycleAction::Quarantine,
            Self::ReleaseQuarantine => SkillCurationLifecycleAction::ReleaseQuarantine,
            Self::Reject => SkillCurationLifecycleAction::Reject,
        }
    }

    /// Stable snake_case label used in trace ids and structured logs.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pin => "pin",
            Self::Unpin => "unpin",
            Self::Archive => "archive",
            Self::Restore => "restore",
            Self::Quarantine => "quarantine",
            Self::ReleaseQuarantine => "release_quarantine",
            Self::Reject => "reject",
        }
    }

    /// HTTP path segment for the public Web Skill operations facade.
    ///
    /// `ReleaseQuarantine` uses kebab-case to match the server route convention.
    pub(crate) fn route_segment(self) -> &'static str {
        match self {
            Self::ReleaseQuarantine => "release-quarantine",
            _ => self.as_str(),
        }
    }
}
