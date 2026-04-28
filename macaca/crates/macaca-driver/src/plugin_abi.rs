//! Driver Plugin ABI - C-ABI stable interface for dynamic driver loading
//!
//! External drivers compiled as shared libraries (.dylib/.so) must implement
//! this ABI to be loadable by the Macaca runtime.

use std::os::raw::{c_char, c_int, c_void};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current ABI version. Bumped on breaking changes.
pub const DRIVER_ABI_VERSION: u32 = 1;

// === Function type aliases for the C-ABI functions ===
// These are the function signatures that shared libraries must export.

/// Returns the ABI version implemented by the driver.
pub type FnDriverAbiVersion = unsafe extern "C" fn() -> u32;

/// Creates a new driver instance from config JSON.
/// Returns an opaque handle, or null on error.
pub type FnDriverCreate = unsafe extern "C" fn(config_json: *const c_char) -> *mut c_void;

/// Returns the driver manifest as a JSON string (caller must free with `FnDriverFreeString`).
pub type FnDriverManifest = unsafe extern "C" fn(handle: *mut c_void) -> *mut c_char;

/// Returns tool definitions as a JSON array string (caller must free with `FnDriverFreeString`).
pub type FnDriverToolDefinitions = unsafe extern "C" fn(handle: *mut c_void) -> *mut c_char;

/// Executes a tool call. Returns result as JSON string (caller must free with `FnDriverFreeString`).
pub type FnDriverExecuteTool = unsafe extern "C" fn(
    handle: *mut c_void,
    tool_name: *const c_char,
    input_json: *const c_char,
) -> *mut c_char;

/// Health check. Returns 1 for healthy, 0 for unhealthy, -1 for error.
pub type FnDriverHealthCheck = unsafe extern "C" fn(handle: *mut c_void) -> c_int;

/// Gracefully shuts down the driver.
pub type FnDriverShutdown = unsafe extern "C" fn(handle: *mut c_void);

/// Destroys the driver instance and frees its memory.
pub type FnDriverDestroy = unsafe extern "C" fn(handle: *mut c_void);

/// Frees a string previously returned by the driver.
pub type FnDriverFreeString = unsafe extern "C" fn(s: *mut c_char);

/// Streaming trace-event callback function type.
///
/// Plugins call this callback during tool execution to push `TraceEvent`s.
/// - `event_json`: JSON-serialized `TraceEvent` string (allocated by plugin, freed by plugin after callback returns)
/// - `user_data`: opaque pointer passed by the host to route events to the correct channel
pub type FnTraceEventCallback = unsafe extern "C" fn(
    event_json: *const c_char,
    user_data: *mut c_void,
);

/// Streaming tool execution — pushes `TraceEvent`s via callback during execution,
/// then returns the final `ToolResultAbi` JSON (same format as `FnDriverExecuteTool`).
///
/// This is an **optional** symbol. v1 drivers that do not export it will fall back
/// to the non-streaming `FnDriverExecuteTool`.
pub type FnDriverExecuteToolStreaming = unsafe extern "C" fn(
    handle: *mut c_void,
    tool_name: *const c_char,
    input_json: *const c_char,
    callback: FnTraceEventCallback,
    user_data: *mut c_void,
) -> *mut c_char;

// === JSON exchange types ===
// These structs are serialized/deserialized across the C-ABI boundary.

/// Tool definition exchanged over the ABI as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinitionAbi {
    pub name: String,
    pub description: String,
    pub parameters_schema: Value,
}

/// Tool execution result exchanged over the ABI as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultAbi {
    pub success: bool,
    pub output: Option<Value>,
    pub error: Option<String>,
}

/// Driver manifest exchanged over the ABI as JSON.
///
/// Maps to the existing `DriverManifest` but with string-based types for ABI stability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverManifestAbi {
    pub name: String,
    pub version: String,
    /// String representation of `DriverType` (e.g. "CliSubprocess", "RestApi").
    pub driver_type: String,
    pub description: String,
    pub capabilities: Vec<String>,
}

// === Standard symbol names ===

/// Standard symbol names that shared libraries must export.
pub mod symbols {
    pub const ABI_VERSION: &str = "macaca_driver_abi_version";
    pub const CREATE: &str = "macaca_driver_create";
    pub const MANIFEST: &str = "macaca_driver_manifest";
    pub const TOOL_DEFINITIONS: &str = "macaca_driver_tool_definitions";
    pub const EXECUTE_TOOL: &str = "macaca_driver_execute_tool";
    pub const HEALTH_CHECK: &str = "macaca_driver_health_check";
    pub const SHUTDOWN: &str = "macaca_driver_shutdown";
    pub const DESTROY: &str = "macaca_driver_destroy";
    pub const FREE_STRING: &str = "macaca_driver_free_string";
    /// Optional symbol for streaming tool execution.
    pub const EXECUTE_TOOL_STREAMING: &str = "macaca_driver_execute_tool_streaming";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_version_is_set() {
        assert_eq!(DRIVER_ABI_VERSION, 1);
    }

    #[test]
    fn tool_definition_abi_roundtrip() {
        let def = ToolDefinitionAbi {
            name: "run_shell".into(),
            description: "Execute a shell command".into(),
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" }
                }
            }),
        };
        let json = serde_json::to_string(&def).unwrap();
        let parsed: ToolDefinitionAbi = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "run_shell");
    }

    #[test]
    fn tool_result_abi_roundtrip() {
        let result = ToolResultAbi {
            success: true,
            output: Some(serde_json::json!({"stdout": "hello"})),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolResultAbi = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert!(parsed.error.is_none());
    }

    #[test]
    fn driver_manifest_abi_roundtrip() {
        let manifest = DriverManifestAbi {
            name: "shell-driver".into(),
            version: "0.1.0".into(),
            driver_type: "CliSubprocess".into(),
            description: "Shell command driver".into(),
            capabilities: vec!["execute_command".into()],
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: DriverManifestAbi = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "shell-driver");
        assert_eq!(parsed.driver_type, "CliSubprocess");
    }

    #[test]
    fn symbol_names_have_prefix() {
        assert!(symbols::ABI_VERSION.starts_with("macaca_driver_"));
        assert!(symbols::CREATE.starts_with("macaca_driver_"));
        assert!(symbols::DESTROY.starts_with("macaca_driver_"));
        assert!(symbols::EXECUTE_TOOL_STREAMING.starts_with("macaca_driver_"));
    }

    #[test]
    fn streaming_symbol_name_is_correct() {
        assert_eq!(
            symbols::EXECUTE_TOOL_STREAMING,
            "macaca_driver_execute_tool_streaming"
        );
    }

    #[test]
    fn callback_and_streaming_fn_types_are_sized() {
        // FnTraceEventCallback is a function pointer — must be pointer-sized
        assert_eq!(
            std::mem::size_of::<FnTraceEventCallback>(),
            std::mem::size_of::<usize>()
        );
        // FnDriverExecuteToolStreaming is a function pointer — must be pointer-sized
        assert_eq!(
            std::mem::size_of::<FnDriverExecuteToolStreaming>(),
            std::mem::size_of::<usize>()
        );
    }
}
