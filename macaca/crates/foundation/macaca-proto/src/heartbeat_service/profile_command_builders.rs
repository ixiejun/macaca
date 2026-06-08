//! Builder helpers for native Heartbeat profile mutation commands.
//!
//! **Pattern:** Builder — fluent profile-policy updates validate cadence/cooldown bounds before a
//! provider persists operator edits, keeping Web/CLI mutation auditable without manifest surgery.

use std::collections::BTreeMap;

use crate::{
    autonomy_service::{non_empty, validate_trace},
    AutonomyScope, MacacaResult, TraceContext,
};

use super::{HeartbeatProfileId, HeartbeatUpdateProfileCommand};

impl HeartbeatUpdateProfileCommand {
    /// Build a profile update command with trace and scope validation.
    pub fn new(
        trace: TraceContext,
        scope: AutonomyScope,
        profile_id: HeartbeatProfileId,
        reason_code: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_trace(&trace, "heartbeat profile update requires trace_id")?;
        Ok(Self {
            trace,
            scope,
            profile_id,
            enabled: None,
            fixed_interval_ms: None,
            cooldown_ms: None,
            metadata: BTreeMap::new(),
            reason_code: non_empty(reason_code.into(), "heartbeat reason_code is required")?,
        })
    }

    /// Attach an optional fixed interval update.
    pub fn with_fixed_interval_ms(mut self, interval_ms: Option<u64>) -> MacacaResult<Self> {
        if matches!(interval_ms, Some(0)) {
            return Err(crate::MacacaError::Config(
                "heartbeat interval must be positive".into(),
            ));
        }
        self.fixed_interval_ms = interval_ms;
        Ok(self)
    }

    /// Attach an optional cooldown update.
    pub fn with_cooldown_ms(mut self, cooldown_ms: Option<u64>) -> MacacaResult<Self> {
        if matches!(cooldown_ms, Some(0)) {
            return Err(crate::MacacaError::Config(
                "heartbeat cooldown must be positive".into(),
            ));
        }
        self.cooldown_ms = cooldown_ms;
        Ok(self)
    }
}
