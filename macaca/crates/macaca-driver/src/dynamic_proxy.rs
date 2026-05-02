//! Proxy for dynamic driver C-ABI calls.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::sync::Arc;

use macaca_proto::{MacacaError, MacacaResult};
use macaca_tools::TraceEvent;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

use crate::command::DriverCommand;
use crate::plugin_abi::*;
use crate::session::DriverSessionState;

/// Resolved C-ABI symbols for a dynamic driver.
pub(crate) struct DynamicDriverSymbols {
    pub(crate) fn_manifest: FnDriverManifest,
    pub(crate) fn_tool_definitions: FnDriverToolDefinitions,
    pub(crate) fn_execute_tool: FnDriverExecuteTool,
    pub(crate) fn_health_check: FnDriverHealthCheck,
    pub(crate) fn_shutdown: FnDriverShutdown,
    pub(crate) fn_destroy: FnDriverDestroy,
    pub(crate) fn_free_string: FnDriverFreeString,
    pub(crate) fn_execute_tool_streaming: Option<FnDriverExecuteToolStreaming>,
}

/// Proxy object that contains all dynamic driver ABI calls.
pub(crate) struct DynamicDriverProxy {
    symbols: DynamicDriverSymbols,
    handle: *mut c_void,
}

// Safety: same C-ABI thread-safety contract as DynamicDriver.
unsafe impl Send for DynamicDriverProxy {}
unsafe impl Sync for DynamicDriverProxy {}

impl DynamicDriverProxy {
    pub(crate) fn new(symbols: DynamicDriverSymbols, handle: *mut c_void) -> Self {
        Self { symbols, handle }
    }

    pub(crate) fn manifest_json(&self) -> MacacaResult<String> {
        let ptr = unsafe { (self.symbols.fn_manifest)(self.handle) };
        if ptr.is_null() {
            return Err(MacacaError::Driver("Driver manifest returned null".into()));
        }
        unsafe { self.read_and_free_string(ptr) }
    }

    pub(crate) fn tool_definitions_json(&self) -> MacacaResult<String> {
        let ptr = unsafe { (self.symbols.fn_tool_definitions)(self.handle) };
        unsafe { self.read_and_free_string(ptr) }
    }

    pub(crate) async fn execute_command(
        self: Arc<Self>,
        command: DriverCommand,
        driver_name: String,
    ) -> MacacaResult<Value> {
        match command {
            DriverCommand::Execute { tool_name, input } => {
                self.execute_non_streaming(tool_name, input).await
            }
            DriverCommand::ExecuteStreaming {
                tool_name,
                input,
                event_tx,
            } => {
                self.execute_streaming_or_fallback(tool_name, input, event_tx, driver_name)
                    .await
            }
        }
    }

    pub(crate) async fn execute_non_streaming(
        self: Arc<Self>,
        tool_name: String,
        input: Value,
    ) -> MacacaResult<Value> {
        let input_json = serde_json::to_string(&input)?;

        tokio::task::spawn_blocking(move || unsafe {
            let tool_name_c = CString::new(tool_name.as_str())
                .map_err(|e| MacacaError::Driver(format!("Tool name contains null byte: {}", e)))?;
            let input_c = CString::new(input_json.as_str()).map_err(|e| {
                MacacaError::Driver(format!("Input JSON contains null byte: {}", e))
            })?;

            let result_ptr =
                (self.symbols.fn_execute_tool)(self.handle, tool_name_c.as_ptr(), input_c.as_ptr());
            self.parse_tool_result_ptr(result_ptr, "Tool execution returned null")
        })
        .await
        .map_err(|e| MacacaError::Driver(format!("Task join error: {}", e)))?
    }

    async fn execute_streaming_or_fallback(
        self: Arc<Self>,
        tool_name: String,
        input: Value,
        event_tx: Option<UnboundedSender<TraceEvent>>,
        driver_name: String,
    ) -> MacacaResult<Value> {
        let fn_streaming = match self.symbols.fn_execute_tool_streaming {
            Some(f) => f,
            None => return self.execute_non_streaming(tool_name, input).await,
        };
        let event_tx = match event_tx {
            Some(tx) => tx,
            None => return self.execute_non_streaming(tool_name, input).await,
        };

        let input_json = serde_json::to_string(&input)?;

        tokio::task::spawn_blocking(move || {
            unsafe extern "C" fn trampoline(event_json: *const c_char, user_data: *mut c_void) {
                if event_json.is_null() || user_data.is_null() {
                    return;
                }
                let session = &*(user_data as *const DriverSessionState);
                let json_str = match CStr::from_ptr(event_json).to_str() {
                    Ok(s) => s,
                    Err(_) => return,
                };
                let _ = session.forward_json_event(json_str);
            }

            let tool_name_c = CString::new(tool_name.as_str())
                .map_err(|e| MacacaError::Driver(format!("Tool name contains null byte: {}", e)))?;
            let input_c = CString::new(input_json.as_str()).map_err(|e| {
                MacacaError::Driver(format!("Input JSON contains null byte: {}", e))
            })?;
            let session = DriverSessionState::new(event_tx, driver_name);
            let user_data = &session as *const DriverSessionState as *mut c_void;

            unsafe {
                let result_ptr = fn_streaming(
                    self.handle,
                    tool_name_c.as_ptr(),
                    input_c.as_ptr(),
                    trampoline,
                    user_data,
                );
                self.parse_tool_result_ptr(result_ptr, "Streaming tool execution returned null")
            }
        })
        .await
        .map_err(|e| MacacaError::Driver(format!("Task join error: {}", e)))?
    }

    pub(crate) fn health_check(&self) -> MacacaResult<bool> {
        let result = unsafe { (self.symbols.fn_health_check)(self.handle) };
        match result {
            1 => Ok(true),
            0 => Ok(false),
            code => Err(MacacaError::Driver(format!(
                "Health check returned error code: {}",
                code
            ))),
        }
    }

    pub(crate) fn shutdown(&self) {
        unsafe {
            (self.symbols.fn_shutdown)(self.handle);
        }
    }

    pub(crate) fn destroy(&self) {
        unsafe {
            (self.symbols.fn_destroy)(self.handle);
        }
    }

    unsafe fn parse_tool_result_ptr(
        &self,
        result_ptr: *mut c_char,
        null_message: &str,
    ) -> MacacaResult<Value> {
        if result_ptr.is_null() {
            return Err(MacacaError::Driver(null_message.into()));
        }

        let result_str = self.read_and_free_string(result_ptr)?;
        let result: ToolResultAbi = serde_json::from_str(&result_str)
            .map_err(|e| MacacaError::Driver(format!("Invalid result JSON: {}", e)))?;

        if result.success {
            Ok(result.output.unwrap_or(Value::Null))
        } else {
            Err(MacacaError::Driver(
                result.error.unwrap_or_else(|| "Unknown tool error".into()),
            ))
        }
    }

    unsafe fn read_and_free_string(&self, ptr: *mut c_char) -> MacacaResult<String> {
        if ptr.is_null() {
            return Err(MacacaError::Driver(
                "Driver returned null string pointer".into(),
            ));
        }
        let res = CStr::from_ptr(ptr)
            .to_str()
            .map(|s| s.to_owned())
            .map_err(|e| MacacaError::Driver(format!("Invalid UTF-8 from driver: {}", e)));
        (self.symbols.fn_free_string)(ptr);
        res
    }
}
