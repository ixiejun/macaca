//! Dynamic driver loading via C-ABI shared libraries.
//!
//! `DynamicDriver` loads a `.dylib`/`.so` at runtime using `libloading`,
//! resolves the standard ABI symbols, and proxies all `SoftwareDriver`
//! method calls through the C-ABI boundary.

use std::ffi::CString;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use libloading::Library;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

use crate::command::DriverCommand;
use crate::driver::{DriverManifest, DriverType, SoftwareDriver};
use crate::dynamic_proxy::{DynamicDriverProxy, DynamicDriverSymbols};
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
    // ── dynamic ABI proxy — dropped before _library ──
    proxy: Arc<DynamicDriverProxy>,

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
    /// Load a driver from a shared library through the canonical dynamic path.
    pub fn load_dynamic(library_path: &Path, config_json: &str) -> MacacaResult<Self> {
        Self::load_internal(library_path, config_json)
    }

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
    #[deprecated(note = "use DynamicDriverFactory or DynamicDriver::load_dynamic()")]
    pub fn load(library_path: &Path, config_json: &str) -> MacacaResult<Self> {
        Self::load_internal(library_path, config_json)
    }

    fn load_internal(library_path: &Path, config_json: &str) -> MacacaResult<Self> {
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
                    MacacaError::Driver(format!("Missing symbol {}: {}", symbols::ABI_VERSION, e))
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

            let fn_execute_tool: FnDriverExecuteTool =
                *library.get(symbols::EXECUTE_TOOL.as_bytes()).map_err(|e| {
                    MacacaError::Driver(format!("Missing symbol {}: {}", symbols::EXECUTE_TOOL, e))
                })?;

            let fn_health_check: FnDriverHealthCheck =
                *library.get(symbols::HEALTH_CHECK.as_bytes()).map_err(|e| {
                    MacacaError::Driver(format!("Missing symbol {}: {}", symbols::HEALTH_CHECK, e))
                })?;

            let fn_shutdown: FnDriverShutdown =
                *library.get(symbols::SHUTDOWN.as_bytes()).map_err(|e| {
                    MacacaError::Driver(format!("Missing symbol {}: {}", symbols::SHUTDOWN, e))
                })?;

            let fn_destroy: FnDriverDestroy =
                *library.get(symbols::DESTROY.as_bytes()).map_err(|e| {
                    MacacaError::Driver(format!("Missing symbol {}: {}", symbols::DESTROY, e))
                })?;

            let fn_free_string: FnDriverFreeString =
                *library.get(symbols::FREE_STRING.as_bytes()).map_err(|e| {
                    MacacaError::Driver(format!("Missing symbol {}: {}", symbols::FREE_STRING, e))
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
            let config_c = CString::new(config_json)
                .map_err(|e| MacacaError::Driver(format!("Config contains null byte: {}", e)))?;
            let handle = fn_create(config_c.as_ptr());
            if handle.is_null() {
                return Err(MacacaError::Driver(
                    "Driver creation returned null handle".into(),
                ));
            }

            let proxy = Arc::new(DynamicDriverProxy::new(
                DynamicDriverSymbols {
                    fn_manifest,
                    fn_tool_definitions,
                    fn_execute_tool,
                    fn_health_check,
                    fn_shutdown,
                    fn_destroy,
                    fn_free_string,
                    fn_execute_tool_streaming,
                },
                handle,
            ));

            // 5. Retrieve and parse the manifest
            let manifest_str = match proxy.manifest_json() {
                Ok(manifest) => manifest,
                Err(e) => {
                    proxy.destroy();
                    return Err(e);
                }
            };

            let manifest_abi: DriverManifestAbi = serde_json::from_str(&manifest_str)
                .map_err(|e| {
                    proxy.destroy();
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
                trace_event_types: manifest_abi.trace_event_types,
            };

            debug!(
                driver = %manifest.name,
                version = %manifest.version,
                "Loaded dynamic driver"
            );

            Ok(Self {
                proxy,
                manifest,
                _library: library,
            })
        }
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
        let defs_json = match self.proxy.tool_definitions_json() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to read tool definitions: {}", e);
                return Vec::new();
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
            proxy: Arc::clone(&self.proxy),
            driver_name: self.manifest.name.clone(),
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
        self.proxy.health_check()
    }

    async fn shutdown(&mut self) -> MacacaResult<()> {
        self.proxy.shutdown();
        Ok(())
    }
}

impl Drop for DynamicDriver {
    fn drop(&mut self) {
        self.proxy.destroy();
    }
}

// ── DynamicTool ─────────────────────────────────────────────────

/// Shared context for tool proxies referencing the same driver instance.
struct DynamicToolContext {
    proxy: Arc<DynamicDriverProxy>,
    driver_name: String,
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

impl DynamicTool {
    async fn execute_via_ffi(&self, input: Value) -> MacacaResult<Value> {
        Arc::clone(&self.ctx.proxy)
            .execute_command(
                DriverCommand::execute(self.name.clone(), input),
                self.ctx.driver_name.clone(),
            )
            .await
    }
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
        self.execute_via_ffi(input).await
    }

    async fn execute_streaming(
        &self,
        input: Value,
        event_tx: Option<UnboundedSender<TraceEvent>>,
    ) -> MacacaResult<Value> {
        Arc::clone(&self.ctx.proxy)
            .execute_command(
                DriverCommand::execute_streaming(self.name.clone(), input, event_tx),
                self.ctx.driver_name.clone(),
            )
            .await
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
