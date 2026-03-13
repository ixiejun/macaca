//! Permission checking for tool execution.

use macaca_proto::{AgentId, MacacaError, MacacaResult, Permission};

/// Checks whether an agent is allowed to execute a given tool.
pub trait PermissionChecker: Send + Sync {
    /// Check if the agent has permission to use the named tool.
    ///
    /// Returns `Ok(())` if allowed, or an error if denied.
    fn check_tool_permission(
        &self,
        agent_id: &AgentId,
        permission: &Permission,
        tool_name: &str,
    ) -> MacacaResult<()>;
}

/// Default permission checker that validates against the `allowed_tools` list.
///
/// If `allowed_tools` is empty, all tools are allowed (open policy).
/// Otherwise, only tools in the list are permitted.
pub struct DefaultPermissionChecker;

impl PermissionChecker for DefaultPermissionChecker {
    fn check_tool_permission(
        &self,
        agent_id: &AgentId,
        permission: &Permission,
        tool_name: &str,
    ) -> MacacaResult<()> {
        // Empty allowed_tools = open policy (all tools allowed)
        if permission.allowed_tools.is_empty() {
            return Ok(());
        }

        if permission.allowed_tools.iter().any(|t| t == tool_name) {
            Ok(())
        } else {
            Err(MacacaError::PermissionDenied(format!(
                "Agent {} is not allowed to use tool '{}'",
                agent_id, tool_name
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::PermissionLevel;

    fn open_permission() -> Permission {
        Permission {
            level: PermissionLevel::User,
            allowed_tools: vec![],
            allowed_paths: vec![],
            network_access: true,
        }
    }

    fn restricted_permission() -> Permission {
        Permission {
            level: PermissionLevel::User,
            allowed_tools: vec!["file_read".into(), "shell".into()],
            allowed_paths: vec![],
            network_access: false,
        }
    }

    #[test]
    fn open_policy_allows_any_tool() {
        let checker = DefaultPermissionChecker;
        let agent_id = AgentId::new();
        let perm = open_permission();
        assert!(checker.check_tool_permission(&agent_id, &perm, "anything").is_ok());
    }

    #[test]
    fn restricted_allows_listed_tool() {
        let checker = DefaultPermissionChecker;
        let agent_id = AgentId::new();
        let perm = restricted_permission();
        assert!(checker.check_tool_permission(&agent_id, &perm, "file_read").is_ok());
        assert!(checker.check_tool_permission(&agent_id, &perm, "shell").is_ok());
    }

    #[test]
    fn restricted_denies_unlisted_tool() {
        let checker = DefaultPermissionChecker;
        let agent_id = AgentId::new();
        let perm = restricted_permission();
        let result = checker.check_tool_permission(&agent_id, &perm, "file_write");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not allowed"));
        assert!(msg.contains("file_write"));
    }
}
