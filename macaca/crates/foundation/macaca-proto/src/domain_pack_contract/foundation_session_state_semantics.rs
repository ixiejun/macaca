//! Provider-neutral approval Specification for session-state side effects.
//!
//! The runtime host supplies only bounded facts.  This keeps approval policy
//! auditable and replayable without exposing keys, state values, checkpoint
//! payloads, or provider handles to the contract layer.

use serde::{Deserialize, Serialize};

/// Sanitized facts used to decide whether a session-state command needs approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateApprovalFacts {
    /// True when a restore targets a different session scope.
    pub cross_session_restore: bool,
    /// True when export can include more than a bounded diagnostic projection.
    pub broad_export: bool,
    /// True when the requested operation can remove or rewrite historical state.
    pub destructive_history_mutation: bool,
    /// Host policy may require approval even for otherwise narrow operations.
    pub policy_requires_approval: bool,
    /// Approval evidence was validated by the policy/approval service.
    pub approval_granted: bool,
}

/// Fail-closed result for a side-effecting command rejected before provider access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStateApprovalFailure {
    ApprovalRequired,
}

/// Determine whether a command must carry host-issued approval evidence.
pub fn requires_session_state_approval(_command: &str, facts: SessionStateApprovalFacts) -> bool {
    (facts.cross_session_restore
        || facts.broad_export
        || facts.destructive_history_mutation
        || facts.policy_requires_approval)
        && !facts.approval_granted
}

/// Enforce approval before provider state can be read or mutated.
pub fn approve_session_state_operation(
    command: &str,
    facts: SessionStateApprovalFacts,
) -> Result<(), SessionStateApprovalFailure> {
    if requires_session_state_approval(command, facts) {
        Err(SessionStateApprovalFailure::ApprovalRequired)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> SessionStateApprovalFacts {
        SessionStateApprovalFacts {
            cross_session_restore: false,
            broad_export: false,
            destructive_history_mutation: false,
            policy_requires_approval: false,
            approval_granted: false,
        }
    }

    #[test]
    fn destructive_commands_fail_closed_without_approval() {
        let mut destructive = facts();
        destructive.destructive_history_mutation = true;
        for command in [
            "session_state.restore_checkpoint",
            "session_state.clear_session",
            "session_state.compact_history",
        ] {
            assert_eq!(
                approve_session_state_operation(command, destructive),
                Err(SessionStateApprovalFailure::ApprovalRequired)
            );
        }
    }

    #[test]
    fn broad_export_and_cross_session_restore_are_approval_gated() {
        let mut export = facts();
        export.broad_export = true;
        assert!(approve_session_state_operation("session_state.export_redacted", export).is_err());

        let mut restore = facts();
        restore.cross_session_restore = true;
        assert!(
            approve_session_state_operation("session_state.restore_checkpoint", restore).is_err()
        );
    }

    #[test]
    fn approved_sensitive_operation_is_admitted() {
        let mut approved = facts();
        approved.destructive_history_mutation = true;
        approved.approval_granted = true;
        assert!(approve_session_state_operation("session_state.clear_session", approved).is_ok());
    }
}
