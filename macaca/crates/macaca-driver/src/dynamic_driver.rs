//! Dynamic driver loading via C-ABI shared libraries.
//!
//! `DynamicDriver` loads a `.dylib`/`.so` at runtime using `libloading`,
//! resolves the standard ABI symbols, and proxies all `SoftwareDriver`
//! method calls through the C-ABI boundary.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use libloading::Library;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

use crate::driver::{DriverManifest, DriverType, SoftwareDriver};
use crate::plugin_abi::*;
use macaca_proto::{DriverId, MacacaError, MacacaResult};
use macaca_tools::{Tool, TraceEvent};

/// A driver loaded from a shared library at runtime.
///
/// # Drop order
///
/// Rust drops struct fields in declaration order. Function pointers
/// reference code inside `_library`, so they **must** be declared
/// (and therefore dropped) **before** `_library`.
pub struct DynamicDriver {
    // ── function pointers — dropped first ──
    #[allow(dead_code)] // kept for drop ordering (must outlive _library)
    fn_manifest: FnDriverManifest,
    fn_tool_definitions: FnDriverToolDefinitions,
    fn_execute_tool: FnDriverExecuteTool,
    fn_health_check: FnDriverHealthCheck,
    fn_shutdown: FnDriverShutdown,
    fn_destroy: FnDriverDestroy,
    fn_free_string: FnDriverFreeString,
    fn_execute_tool_streaming: Option<FnDriverExecuteToolStreaming>,

    // ── driver instance handle ──
    handle: *mut c_void,

    // ── cached manifest ──
    manifest: DriverManifest,

    // ── library handle — MUST be dropped last ──
    _library: Library,
}

// Safety: The C-ABI contract guarantees the driver implementation is
// thread-safe. The opaque handle is only accessed through the resolved
// function pointers which themselves are `Send + Sync` by convention.
unsafe impl Send for DynamicDriver {}
unsafe impl Sync for DynamicDriver {}

impl DynamicDriver {
    /// Load a driver from a shared library.
    ///
    /// # Arguments
    /// * `library_path` — Path to the `.dylib` / `.so` file.
    /// * `config_json`  — JSON configuration forwarded to the driver's
    ///   `macaca_driver_create` entry point.
    ///
    /// # Errors
    /// Returns `MacacaError::Driver` if the library cannot be loaded,
    /// required symbols are missing, the ABI version is incompatible,
    /// or the driver fails to initialize.
    pub fn load(library_path: &Path, config_json: &str) -> MacacaResult<Self> {
        unsafe {
            // 1. Load the shared library
            let library = Library::new(library_path).map_err(|e| {
                MacacaError::Driver(format!(
                    "Failed to load driver library {:?}: {}",
                    library_path, e
                ))
            })?;

            // 2. Resolve all required symbols
            let fn_abi_version: FnDriverAbiVersion =
                *library.get(symbols::ABI_VERSION.as_bytes()).map_err(|e| {
                    MacacaError::Driver(format!(
                        "Missing symbol {}: {}",
                        symbols::ABI_VERSION,
                        e
                    ))
                })?;

            let fn_create: FnDriverCreate =
                *library.get(symbols::CREATE.as_bytes()).map_err(|e| {
                    MacacaError::Driver(format!("Missing symbol {}: {}", symbols::CREATE, e))
                })?;

            let fn_manifest: FnDriverManifest =
                *library.get(symbols::MANIFEST.as_bytes()).map_err(|e| {
                    MacacaError::Driver(format!("Missing symbol {}: {}", symbols::MANIFEST, e))
                })?;

            let fn_tool_definitions: FnDriverToolDefinitions = *library
                .get(symbols::TOOL_DEFINITIONS.as_bytes())
                .map_err(|e| {
                    MacacaError::Driver(format!(
                        "Missing symbol {}: {}",
                        symbols::TOOL_DEFINITIONS,
                        e
                    ))
                })?;

            let fn_execute_tool: FnDriverExecuteTool = *library
                .get(symbols::EXECUTE_TOOL.as_bytes())
                .map_err(|e| {
                    MacacaError::Driver(format!(
                        "Missing symbol {}: {}",
                        symbols::EXECUTE_TOOL,
                        e
                    ))
                })?;

            let fn_health_check: FnDriverHealthCheck = *library
                .get(symbols::HEALTH_CHECK.as_bytes())
                .map_err(|e| {
                    MacacaError::Driver(format!(
                        "Missing symbol {}: {}",
                        symbols::HEALTH_CHECK,
                        e
                    ))
                })?;

            let fn_shutdown: FnDriverShutdown =
                *library.get(symbols::SHUTDOWN.as_bytes()).map_err(|e| {
                    MacacaError::Driver(format!("Missing symbol {}: {}", symbols::SHUTDOWN, e))
                })?;

            let fn_destroy: FnDriverDestroy =
                *library.get(symbols::DESTROY.as_bytes()).map_err(|e| {
                    MacacaError::Driver(format!("Missing symbol {}: {}", symbols::DESTROY, e))
                })?;

            let fn_free_string: FnDriverFreeString = *library
                .get(symbols::FREE_STRING.as_bytes())
                .map_err(|e| {
                    MacacaError::Driver(format!(
                        "Missing symbol {}: {}",
                        symbols::FREE_STRING,
                        e
                    ))
                })?;

            // Optional: load streaming execution symbol (not required for v1 drivers)
            let fn_execute_tool_streaming: Option<FnDriverExecuteToolStreaming> = library
                .get::<FnDriverExecuteToolStreaming>(symbols::EXECUTE_TOOL_STREAMING.as_bytes())
                .ok()
                .map(|sym| *sym);

            // 3. Verify ABI version compatibility
            let abi_version = fn_abi_version();
            if abi_version < DRIVER_ABI_VERSION {
                return Err(MacacaError::Driver(format!(
                    "Driver ABI version {} is older than required {}",
                    abi_version, DRIVER_ABI_VERSION
                )));
            }

            // 4. Create driver instance
            let config_c = CString::new(config_json).map_err(|e| {
                MacacaError::Driver(format!("Config contains null byte: {}", e))
            })?;
            let handle = fn_create(config_c.as_ptr());
            if handle.is_null() {
                return Err(MacacaError::Driver(
                    "Driver creation returned null handle".into(),
                ));
            }

            // 5. Retrieve and parse the manifest
            let manifest_ptr = fn_manifest(handle);
            if manifest_ptr.is_null() {
                fn_destroy(handle);
                return Err(MacacaError::Driver(
                    "Driver manifest returned null".into(),
                ));
            }
            let manifest_res = CStr::from_ptr(manifest_ptr)
                .to_str()
                .map(|s| s.to_owned())
                .map_err(|e| {
                    MacacaError::Driver(format!("Invalid UTF-8 in manifest: {}", e))
                });
            fn_free_string(manifest_ptr); // always free, even on UTF-8 error
            let manifest_str = manifest_res?;

            let manifest_abi: DriverManifestAbi =
                serde_json::from_str(&manifest_str).map_err(|e| {
                    MacacaError::Driver(format!("Invalid manifest JSON: {}", e))
                })?;

            // 6. Convert ABI manifest → domain DriverManifest
            let driver_type = parse_driver_type(&manifest_abi.driver_type)?;
            let manifest = DriverManifest {
                id: DriverId::new(),
                name: manifest_abi.name,
                version: manifest_abi.version,
                driver_type,
                description: manifest_abi.description,
                capabilities: manifest_abi.capabilities,
            };

            debug!(
                driver = %manifest.name,
                version = %manifest.version,
                "Loaded dynamic driver"
            );

            Ok(Self {
                fn_manifest,
                fn_tool_definitions,
                fn_execute_tool,
                fn_health_check,
                fn_shutdown,
                fn_destroy,
                fn_free_string,
                fn_execute_tool_streaming,
                handle,
                manifest,
                _library: library,
            })
        }
    }

    /// Read a C string returned by the driver and free it.
    ///
    /// Returns `Err` if the pointer is null or the string is not valid UTF-8.
    /// The string buffer is always freed (via `fn_free_string`) on non-null
    /// pointers, even when UTF-8 decoding fails.
    unsafe fn read_and_free_string(&self, ptr: *mut std::os::raw::c_char) -> MacacaResult<String> {
        if ptr.is_null() {
            return Err(MacacaError::Driver(
                "Driver returned null string pointer".into(),
            ));
        }
        let res = CStr::from_ptr(ptr)
            .to_str()
            .map(|s| s.to_owned())
            .map_err(|e| MacacaError::Driver(format!("Invalid UTF-8 from driver: {}", e)));
        (self.fn_free_string)(ptr); // always free, even on UTF-8 error
        res
    }
}

#[async_trait]
impl SoftwareDriver for DynamicDriver {
    fn manifest(&self) -> &DriverManifest {
        &self.manifest
    }

    async fn initialize(&mut self) -> MacacaResult<()> {
        // Already initialized during load()
        Ok(())
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        let defs_json = unsafe {
            let ptr = (self.fn_tool_definitions)(self.handle);
            match self.read_and_free_string(ptr) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to read tool definitions: {}", e);
                    return Vec::new();
                }
            }
        };

        let definitions: Vec<ToolDefinitionAbi> = match serde_json::from_str(&defs_json) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to parse tool definitions JSON: {}", e);
                return Vec::new();
            }
        };

        let ctx = Arc::new(DynamicToolContext {
            handle: self.handle,
            fn_execute_tool: self.fn_execute_tool,
            fn_free_string: self.fn_free_string,
            fn_execute_tool_streaming: self.fn_execute_tool_streaming,
        });

        definitions
            .into_iter()
            .map(|def| {
                Box::new(DynamicTool {
                    name: def.name,
                    description: def.description,
                    parameters_schema: def.parameters_schema,
                    ctx: Arc::clone(&ctx),
                }) as Box<dyn Tool>
            })
            .collect()
    }

    async fn health_check(&self) -> MacacaResult<bool> {
        let result = unsafe { (self.fn_health_check)(self.handle) };
        match result {
            1 => Ok(true),
            0 => Ok(false),
            code => Err(MacacaError::Driver(format!(
                "Health check returned error code: {}",
                code
            ))),
        }
    }

    async fn shutdown(&mut self) -> MacacaResult<()> {
        unsafe {
            (self.fn_shutdown)(self.handle);
        }
        Ok(())
    }
}

impl Drop for DynamicDriver {
    fn drop(&mut self) {
        unsafe {
            (self.fn_destroy)(self.handle);
        }
    }
}

// ── DynamicTool ─────────────────────────────────────────────────

/// Shared context for tool proxies referencing the same driver instance.
struct DynamicToolContext {
    handle: *mut c_void,
    fn_execute_tool: FnDriverExecuteTool,
    fn_free_string: FnDriverFreeString,
    fn_execute_tool_streaming: Option<FnDriverExecuteToolStreaming>,
}

// Safety: same C-ABI thread-safety contract as DynamicDriver.
unsafe impl Send for DynamicToolContext {}
unsafe impl Sync for DynamicToolContext {}

/// A tool proxy that forwards execution to the loaded driver via C-ABI.
pub struct DynamicTool {
    name: String,
    description: String,
    parameters_schema: Value,
    ctx: Arc<DynamicToolContext>,
}

#[async_trait]
impl Tool for DynamicTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        self.parameters_schema.clone()
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let ctx = Arc::clone(&self.ctx);
        let tool_name = self.name.clone();
        let input_json = serde_json::to_string(&input)?;

        // Bridge synchronous FFI call into async runtime via spawn_blocking
        tokio::task::spawn_blocking(move || unsafe {
            let tool_name_c = CString::new(tool_name.as_str()).map_err(|e| {
                MacacaError::Driver(format!("Tool name contains null byte: {}", e))
            })?;
            let input_c = CString::new(input_json.as_str()).map_err(|e| {
                MacacaError::Driver(format!("Input JSON contains null byte: {}", e))
            })?;

            let result_ptr =
                (ctx.fn_execute_tool)(ctx.handle, tool_name_c.as_ptr(), input_c.as_ptr());

            if result_ptr.is_null() {
                return Err(MacacaError::Driver(
                    "Tool execution returned null".into(),
                ));
            }

            let result_res = CStr::from_ptr(result_ptr)
                .to_str()
                .map(|s| s.to_owned())
                .map_err(|e| MacacaError::Driver(format!("Invalid UTF-8 in result: {}", e)));
            (ctx.fn_free_string)(result_ptr); // always free, even on UTF-8 error
            let result_str = result_res?;

            let result: ToolResultAbi = serde_json::from_str(&result_str)
                .map_err(|e| MacacaError::Driver(format!("Invalid result JSON: {}", e)))?;

            if result.success {
                Ok(result.output.unwrap_or(Value::Null))
            } else {
                Err(MacacaError::Driver(
                    result.error.unwrap_or_else(|| "Unknown tool error".into()),
                ))
            }
        })
        .await
        .map_err(|e| MacacaError::Driver(format!("Task join error: {}", e)))?
    }

    async fn execute_streaming(
        &self,
        input: Value,
        event_tx: Option<UnboundedSender<TraceEvent>>,
    ) -> MacacaResult<Value> {
        // Check if the driver exports the streaming FFI symbol
        let fn_streaming = match self.ctx.fn_execute_tool_streaming {
            Some(f) => f,
            None => {
                // Fall back to non-streaming execute()
                return self.execute(input).await;
            }
        };

        // If no event sender, also fall back to non-streaming
        let event_tx = match event_tx {
            Some(tx) => tx,
            None => return self.execute(input).await,
        };

        let ctx = Arc::clone(&self.ctx);
        let tool_name = self.name.clone();
        let input_json = serde_json::to_string(&input)?;

        tokio::task::spawn_blocking(move || {
            // Trampoline callback: pure extern "C" fn, routes events via user_data
            unsafe extern "C" fn trampoline(
                event_json: *const c_char,
                user_data: *mut c_void,
            ) {
                if event_json.is_null() || user_data.is_null() {
                    return;
                }
                let tx = &*(user_data as *const UnboundedSender<TraceEvent>);
                let json_str = match CStr::from_ptr(event_json).to_str() {
                    Ok(s) => s,
                    Err(_) => return,
                };
                if let Ok(event) = serde_json::from_str::<TraceEvent>(json_str) {
                    let _ = tx.send(event);
                }
            }

            let tool_name_c = CString::new(tool_name.as_str()).map_err(|e| {
                MacacaError::Driver(format!("Tool name contains null byte: {}", e))
            })?;
            let input_c = CString::new(input_json.as_str()).map_err(|e| {
                MacacaError::Driver(format!("Input JSON contains null byte: {}", e))
            })?;

            // user_data points to event_tx, which lives for the duration of this closure
            let user_data = &event_tx as *const UnboundedSender<TraceEvent> as *mut c_void;

            unsafe {
                let result_ptr = fn_streaming(
                    ctx.handle,
                    tool_name_c.as_ptr(),
                    input_c.as_ptr(),
                    trampoline,
                    user_data,
                );

                if result_ptr.is_null() {
                    return Err(MacacaError::Driver(
                        "Streaming tool execution returned null".into(),
                    ));
                }

                let result_res = CStr::from_ptr(result_ptr)
                    .to_str()
                    .map(|s| s.to_owned())
                    .map_err(|e| {
                        MacacaError::Driver(format!("Invalid UTF-8 in result: {}", e))
                    });
                (ctx.fn_free_string)(result_ptr); // always free, even on UTF-8 error
                let result_str = result_res?;

                let result: ToolResultAbi = serde_json::from_str(&result_str).map_err(|e| {
                    MacacaError::Driver(format!("Invalid result JSON: {}", e))
                })?;

                if result.success {
                    Ok(result.output.unwrap_or(Value::Null))
                } else {
                    Err(MacacaError::Driver(
                        result.error.unwrap_or_else(|| "Unknown tool error".into()),
                    ))
                }
            }
        })
        .await
        .map_err(|e| MacacaError::Driver(format!("Task join error: {}", e)))?
    }
}

// ── Helpers ─────────────────────────────────────────────────────

/// Parse a string driver type from the ABI manifest into `DriverType`.
fn parse_driver_type(s: &str) -> MacacaResult<DriverType> {
    match s {
        "CliSubprocess" => Ok(DriverType::CliSubprocess),
        "RestApi" => Ok(DriverType::RestApi),
        "UiAutomation" => Ok(DriverType::UiAutomation),
        "FileIpc" => Ok(DriverType::FileIpc),
        "McpProtocol" => Ok(DriverType::McpProtocol),
        other => Err(MacacaError::Driver(format!(
            "Unknown driver type: {}",
            other
        ))),
    }
}
