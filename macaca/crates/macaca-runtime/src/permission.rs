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

    /// Check if the agent has permission to use the named tool with the given arguments.
    ///
    /// The default implementation ignores arguments and delegates to `check_tool_permission`.
    /// Override this to add path or network access enforcement.
    fn check_tool_with_args(
        &self,
        agent_id: &AgentId,
        permission: &Permission,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> MacacaResult<()> {
        let _ = arguments;
        self.check_tool_permission(agent_id, permission, tool_name)
    }
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

    /// Check if tool execution is allowed, considering both tool name and arguments.
    ///
    /// Extends `check_tool_permission` with path and network access enforcement.
    fn check_tool_with_args(
        &self,
        agent_id: &AgentId,
        permission: &Permission,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> MacacaResult<()> {
        // 1. Existing tool name check
        self.check_tool_permission(agent_id, permission, tool_name)?;

        // 2. Path permission check for file tools
        if is_file_tool(tool_name) {
            if let Some(path) = extract_path_from_args(arguments) {
                check_path_allowed(&path, &permission.allowed_paths)?;
            }
        }

        // 3. Network access check for shell/network tools
        if is_network_tool(tool_name, arguments) && !permission.network_access {
            return Err(MacacaError::PermissionDenied(format!(
                "Agent '{}' does not have network access permission",
                agent_id
            )));
        }

        Ok(())
    }
}

/// Check if a tool operates on files.
fn is_file_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "file_read"
            | "file_write"
            | "read_file"
            | "write_file"
            | "file_edit"
            | "file_append"
            | "list_directory"
    )
}

/// Extract file path from tool arguments.
///
/// Tries common key names: `path`, `file_path`, `directory`.
fn extract_path_from_args(args: &serde_json::Value) -> Option<String> {
    args.get("path")
        .or_else(|| args.get("file_path"))
        .or_else(|| args.get("directory"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Check if a path is within the allowed paths using prefix matching.
///
/// If `allowed_paths` is empty, all paths are permitted (no restriction).
fn check_path_allowed(path: &str, allowed_paths: &[String]) -> MacacaResult<()> {
    if allowed_paths.is_empty() {
        return Ok(());
    }

    let canonical = std::path::Path::new(path).to_string_lossy();

    for allowed in allowed_paths {
        if canonical.starts_with(allowed.as_str()) {
            return Ok(());
        }
    }

    Err(MacacaError::PermissionDenied(format!(
        "Path '{}' is not within allowed paths: {:?}",
        path, allowed_paths
    )))
}

/// Check if a tool or its arguments involve network access.
fn is_network_tool(tool_name: &str, args: &serde_json::Value) -> bool {
    // Explicit network tools
    if matches!(tool_name, "http_request" | "fetch" | "web_search") {
        return true;
    }
    // Shell commands that may involve network
    if matches!(tool_name, "shell" | "bash" | "execute_command") {
        if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
            let lower = cmd.to_lowercase();
            return lower.contains("curl ")
                || lower.contains("wget ")
                || lower.contains("nc ")
                || lower.contains("ssh ")
                || lower.contains("scp ")
                || lower.contains("rsync ")
                || lower.starts_with("curl")
                || lower.starts_with("wget");
        }
    }
    false
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
        assert!(checker
            .check_tool_permission(&agent_id, &perm, "anything")
            .is_ok());
    }

    #[test]
    fn restricted_allows_listed_tool() {
        let checker = DefaultPermissionChecker;
        let agent_id = AgentId::new();
        let perm = restricted_permission();
        assert!(checker
            .check_tool_permission(&agent_id, &perm, "file_read")
            .is_ok());
        assert!(checker
            .check_tool_permission(&agent_id, &perm, "shell")
            .is_ok());
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

    // ── check_tool_with_args: path permission tests ──

    #[test]
    fn test_path_allowed() {
        let checker = DefaultPermissionChecker;
        let agent_id = AgentId::new();
        let perm = Permission {
            level: PermissionLevel::User,
            allowed_tools: vec![],
            allowed_paths: vec!["/home/agent".into()],
            network_access: false,
        };
        let args = serde_json::json!({"path": "/home/agent/data.txt"});
        assert!(checker
            .check_tool_with_args(&agent_id, &perm, "file_read", &args)
            .is_ok());
    }

    #[test]
    fn test_path_denied() {
        let checker = DefaultPermissionChecker;
        let agent_id = AgentId::new();
        let perm = Permission {
            level: PermissionLevel::User,
            allowed_tools: vec![],
            allowed_paths: vec!["/home/agent".into()],
            network_access: false,
        };
        let args = serde_json::json!({"path": "/etc/passwd"});
        let result = checker.check_tool_with_args(&agent_id, &perm, "file_read", &args);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("/etc/passwd"));
        assert!(msg.contains("not within allowed paths"));
    }

    #[test]
    fn test_empty_paths_allows_all() {
        let checker = DefaultPermissionChecker;
        let agent_id = AgentId::new();
        let perm = Permission {
            level: PermissionLevel::User,
            allowed_tools: vec![],
            allowed_paths: vec![],
            network_access: false,
        };
        let args = serde_json::json!({"path": "/etc/passwd"});
        // Empty allowed_paths = no path restriction
        assert!(checker
            .check_tool_with_args(&agent_id, &perm, "file_read", &args)
            .is_ok());
    }

    // ── check_tool_with_args: network permission tests ──

    #[test]
    fn test_network_denied() {
        let checker = DefaultPermissionChecker;
        let agent_id = AgentId::new();
        let perm = Permission {
            level: PermissionLevel::User,
            allowed_tools: vec![],
            allowed_paths: vec![],
            network_access: false,
        };
        let args = serde_json::json!({"command": "curl https://example.com"});
        let result = checker.check_tool_with_args(&agent_id, &perm, "shell", &args);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("network access permission"));
    }

    #[test]
    fn test_network_allowed() {
        let checker = DefaultPermissionChecker;
        let agent_id = AgentId::new();
        let perm = Permission {
            level: PermissionLevel::User,
            allowed_tools: vec![],
            allowed_paths: vec![],
            network_access: true,
        };
        let args = serde_json::json!({"command": "curl https://example.com"});
        assert!(checker
            .check_tool_with_args(&agent_id, &perm, "shell", &args)
            .is_ok());
    }

    #[test]
    fn test_non_network_shell_allowed() {
        let checker = DefaultPermissionChecker;
        let agent_id = AgentId::new();
        let perm = Permission {
            level: PermissionLevel::User,
            allowed_tools: vec![],
            allowed_paths: vec![],
            network_access: false,
        };
        // Plain `ls` has no network activity — should pass even with network_access=false
        let args = serde_json::json!({"command": "ls /tmp"});
        assert!(checker
            .check_tool_with_args(&agent_id, &perm, "shell", &args)
            .is_ok());
    }
}
