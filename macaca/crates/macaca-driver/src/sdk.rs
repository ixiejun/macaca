//! Driver SDK — convenience macro for exporting a `SoftwareDriver` as a C-ABI plugin.
//!
//! Third-party driver authors use `export_driver!` so they only write the
//! `SoftwareDriver` impl; the macro generates every `#[no_mangle] extern "C"`
//! symbol required by the plugin ABI.
//!
//! # Example
//!
//! ```rust,ignore
//! macaca_driver::export_driver!(MyDriver, |config: serde_json::Value| {
//!     let cfg: MyConfig = serde_json::from_value(config)?;
//!     Ok(Box::new(MyDriver::new(cfg)))
//! });
//! ```

/// Generate all C-ABI entry points for a [`SoftwareDriver`](crate::driver::SoftwareDriver)
/// implementation.
///
/// # Arguments
///
/// * `$driver_type` — the concrete driver type (used only for documentation /
///   error messages; the factory closure is what actually builds it).
/// * `$create_fn` — a closure `fn(serde_json::Value) -> Result<Box<dyn
///   SoftwareDriver>, Box<dyn std::error::Error>>` that constructs the driver
///   from a JSON config blob.
#[macro_export]
macro_rules! export_driver {
    ($driver_type:ty, $create_fn:expr) => {
        // ── global driver storage ──────────────────────────────────────
        //
        // We store the single driver instance behind a Mutex so every
        // extern "C" function can access it safely.  The `handle` pointer
        // returned by `macaca_driver_create` is a non-null sentinel — the
        // real object lives here.

        static DRIVER_INSTANCE: std::sync::Mutex<Option<Box<dyn $crate::driver::SoftwareDriver>>> =
            std::sync::Mutex::new(None);

        // ── helpers ────────────────────────────────────────────────────

        /// Small helper: try to serialize a JSON-serializable value to a
        /// `CString` and leak it as `*mut c_char`. Returns null on failure.
        fn _sdk_json_to_c_char(
            json_string: Result<String, serde_json::Error>,
        ) -> *mut std::os::raw::c_char {
            match json_string {
                Ok(json) => match std::ffi::CString::new(json) {
                    Ok(c) => c.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                },
                Err(_) => std::ptr::null_mut(),
            }
        }

        // ── ABI version ────────────────────────────────────────────────

        #[no_mangle]
        pub extern "C" fn macaca_driver_abi_version() -> u32 {
            $crate::plugin_abi::DRIVER_ABI_VERSION
        }

        // ── create ─────────────────────────────────────────────────────

        #[no_mangle]
        pub extern "C" fn macaca_driver_create(
            config_json: *const std::os::raw::c_char,
        ) -> *mut std::os::raw::c_void {
            std::panic::catch_unwind(|| {
                if config_json.is_null() {
                    return std::ptr::null_mut();
                }
                let config_str = match unsafe { std::ffi::CStr::from_ptr(config_json) }.to_str() {
                    Ok(s) => s,
                    Err(_) => return std::ptr::null_mut(),
                };
                let config: serde_json::Value = match serde_json::from_str(config_str) {
                    Ok(v) => v,
                    Err(_) => return std::ptr::null_mut(),
                };

                let create: fn(
                    serde_json::Value,
                ) -> Result<
                    Box<dyn $crate::driver::SoftwareDriver>,
                    Box<dyn std::error::Error>,
                > = $create_fn;

                match create(config) {
                    Ok(driver) => {
                        let mut guard = DRIVER_INSTANCE.lock().unwrap();
                        *guard = Some(driver);
                        // Return a non-null sentinel (we use global storage)
                        1usize as *mut std::os::raw::c_void
                    }
                    Err(_) => std::ptr::null_mut(),
                }
            })
            .unwrap_or(std::ptr::null_mut())
        }

        // ── manifest ───────────────────────────────────────────────────

        #[no_mangle]
        pub extern "C" fn macaca_driver_manifest(
            handle: *mut std::os::raw::c_void,
        ) -> *mut std::os::raw::c_char {
            std::panic::catch_unwind(|| {
                if handle.is_null() {
                    return std::ptr::null_mut();
                }
                let guard = DRIVER_INSTANCE.lock().unwrap();
                if let Some(ref driver) = *guard {
                    let m = driver.manifest();
                    let abi = $crate::plugin_abi::DriverManifestAbi {
                        name: m.name.clone(),
                        version: m.version.clone(),
                        driver_type: format!("{:?}", m.driver_type),
                        description: m.description.clone(),
                        capabilities: m.capabilities.clone(),
                        trace_event_types: m.trace_event_types.clone(),
                    };
                    _sdk_json_to_c_char(serde_json::to_string(&abi))
                } else {
                    std::ptr::null_mut()
                }
            })
            .unwrap_or(std::ptr::null_mut())
        }

        // ── tool definitions ───────────────────────────────────────────

        #[no_mangle]
        pub extern "C" fn macaca_driver_tool_definitions(
            handle: *mut std::os::raw::c_void,
        ) -> *mut std::os::raw::c_char {
            std::panic::catch_unwind(|| {
                if handle.is_null() {
                    return std::ptr::null_mut();
                }
                let guard = DRIVER_INSTANCE.lock().unwrap();
                if let Some(ref driver) = *guard {
                    let defs: Vec<$crate::plugin_abi::ToolDefinitionAbi> = driver
                        .tools()
                        .iter()
                        .map(|t| $crate::plugin_abi::ToolDefinitionAbi {
                            name: t.name().to_string(),
                            description: t.description().to_string(),
                            parameters_schema: t.parameters_schema(),
                        })
                        .collect();
                    _sdk_json_to_c_char(serde_json::to_string(&defs))
                } else {
                    std::ptr::null_mut()
                }
            })
            .unwrap_or(std::ptr::null_mut())
        }

        // ── execute tool ───────────────────────────────────────────────

        #[no_mangle]
        pub extern "C" fn macaca_driver_execute_tool(
            handle: *mut std::os::raw::c_void,
            tool_name: *const std::os::raw::c_char,
            input_json: *const std::os::raw::c_char,
        ) -> *mut std::os::raw::c_char {
            std::panic::catch_unwind(|| {
                if handle.is_null() || tool_name.is_null() || input_json.is_null() {
                    return std::ptr::null_mut();
                }

                let tool_name_str = match unsafe { std::ffi::CStr::from_ptr(tool_name) }.to_str() {
                    Ok(s) => s,
                    Err(_) => return std::ptr::null_mut(),
                };
                let input_str = match unsafe { std::ffi::CStr::from_ptr(input_json) }.to_str() {
                    Ok(s) => s,
                    Err(_) => return std::ptr::null_mut(),
                };
                let input: serde_json::Value = serde_json::from_str(input_str).unwrap_or_default();

                let guard = DRIVER_INSTANCE.lock().unwrap();
                if let Some(ref driver) = *guard {
                    let tools = driver.tools();
                    let tool = tools.iter().find(|t| t.name() == tool_name_str);

                    let result = if let Some(tool) = tool {
                        // Build a lightweight current-thread runtime to bridge
                        // the async Tool::execute into this synchronous FFI fn.
                        match tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                        {
                            Ok(rt) => match rt.block_on(tool.execute(input)) {
                                Ok(output) => $crate::plugin_abi::ToolResultAbi {
                                    success: true,
                                    output: Some(output),
                                    error: None,
                                },
                                Err(e) => $crate::plugin_abi::ToolResultAbi {
                                    success: false,
                                    output: None,
                                    error: Some(e.to_string()),
                                },
                            },
                            Err(e) => $crate::plugin_abi::ToolResultAbi {
                                success: false,
                                output: None,
                                error: Some(format!("Failed to create runtime: {}", e)),
                            },
                        }
                    } else {
                        $crate::plugin_abi::ToolResultAbi {
                            success: false,
                            output: None,
                            error: Some(format!("Tool '{}' not found", tool_name_str)),
                        }
                    };

                    _sdk_json_to_c_char(serde_json::to_string(&result))
                } else {
                    std::ptr::null_mut()
                }
            })
            .unwrap_or(std::ptr::null_mut())
        }

        // ── execute tool streaming ────────────────────────────────────

        #[no_mangle]
        pub extern "C" fn macaca_driver_execute_tool_streaming(
            handle: *mut std::os::raw::c_void,
            tool_name: *const std::os::raw::c_char,
            input_json: *const std::os::raw::c_char,
            callback: $crate::plugin_abi::FnTraceEventCallback,
            user_data: *mut std::os::raw::c_void,
        ) -> *mut std::os::raw::c_char {
            // Wrapper to safely send FFI callback pointers across threads.
            struct _FfiCallbackCtx {
                callback: $crate::plugin_abi::FnTraceEventCallback,
                user_data: *mut std::os::raw::c_void,
            }
            unsafe impl Send for _FfiCallbackCtx {}

            std::panic::catch_unwind(|| {
                if handle.is_null() || tool_name.is_null() || input_json.is_null() {
                    return std::ptr::null_mut();
                }

                let tool_name_str = match unsafe { std::ffi::CStr::from_ptr(tool_name) }.to_str() {
                    Ok(s) => s,
                    Err(_) => return std::ptr::null_mut(),
                };
                let input_str = match unsafe { std::ffi::CStr::from_ptr(input_json) }.to_str() {
                    Ok(s) => s,
                    Err(_) => return std::ptr::null_mut(),
                };
                let input: serde_json::Value = serde_json::from_str(input_str).unwrap_or_default();

                let guard = DRIVER_INSTANCE.lock().unwrap();
                if let Some(ref driver) = *guard {
                    let tools = driver.tools();
                    let tool = tools.iter().find(|t| t.name() == tool_name_str);

                    let result = if let Some(tool) = tool {
                        // Create an unbounded channel for TraceEvents
                        let (trace_tx, mut trace_rx) =
                            tokio::sync::mpsc::unbounded_channel::<$crate::TraceEvent>();

                        let ctx = _FfiCallbackCtx {
                            callback,
                            user_data,
                        };

                        // Dedicated OS thread for forwarding trace events.
                        // Uses blocking_recv() so events are forwarded immediately
                        // without depending on tokio scheduling.
                        //
                        // SAFETY: The spawned thread is joined before this
                        // function returns, so all captured pointers remain valid.
                        let fwd_closure = $crate::SendableFn(move || {
                            while let Some(event) = trace_rx.blocking_recv() {
                                if let Ok(json) = serde_json::to_string(&event) {
                                    if let Ok(cstr) = std::ffi::CString::new(json) {
                                        unsafe {
                                            (ctx.callback)(cstr.as_ptr(), ctx.user_data);
                                        }
                                    }
                                }
                            }
                        });
                        let fwd_thread = std::thread::spawn(move || fwd_closure.call());

                        // Build a single-threaded runtime and use LocalSet so that
                        // the tool execution can use spawn_local if needed.
                        match tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                        {
                            Ok(rt) => {
                                let local = tokio::task::LocalSet::new();

                                // Run the tool execution on the current-thread
                                // runtime. trace_tx is moved into execute_streaming
                                // and will be dropped when it completes, causing
                                // the forwarding thread to exit.
                                let exec_result = rt.block_on(
                                    local.run_until(tool.execute_streaming(input, Some(trace_tx))),
                                );

                                // Wait for forwarding thread to drain all remaining events
                                let _ = fwd_thread.join();

                                match exec_result {
                                    Ok(output) => $crate::plugin_abi::ToolResultAbi {
                                        success: true,
                                        output: Some(output),
                                        error: None,
                                    },
                                    Err(e) => $crate::plugin_abi::ToolResultAbi {
                                        success: false,
                                        output: None,
                                        error: Some(e.to_string()),
                                    },
                                }
                            }
                            Err(e) => {
                                // Runtime creation failed; drop trace_tx so the
                                // forwarding thread can exit, then join it.
                                drop(trace_tx);
                                let _ = fwd_thread.join();
                                $crate::plugin_abi::ToolResultAbi {
                                    success: false,
                                    output: None,
                                    error: Some(format!("Failed to create runtime: {}", e)),
                                }
                            }
                        }
                    } else {
                        $crate::plugin_abi::ToolResultAbi {
                            success: false,
                            output: None,
                            error: Some(format!("Tool '{}' not found", tool_name_str)),
                        }
                    };

                    _sdk_json_to_c_char(serde_json::to_string(&result))
                } else {
                    std::ptr::null_mut()
                }
            })
            .unwrap_or(std::ptr::null_mut())
        }

        // ── health check ───────────────────────────────────────────────

        #[no_mangle]
        pub extern "C" fn macaca_driver_health_check(
            handle: *mut std::os::raw::c_void,
        ) -> std::os::raw::c_int {
            std::panic::catch_unwind(|| {
                if handle.is_null() {
                    return -1;
                }
                let guard = DRIVER_INSTANCE.lock().unwrap();
                if let Some(ref driver) = *guard {
                    match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(rt) => match rt.block_on(driver.health_check()) {
                            Ok(true) => 1,
                            Ok(false) => 0,
                            Err(_) => -1,
                        },
                        Err(_) => -1,
                    }
                } else {
                    -1
                }
            })
            .unwrap_or(-1)
        }

        // ── shutdown ───────────────────────────────────────────────────

        #[no_mangle]
        pub extern "C" fn macaca_driver_shutdown(handle: *mut std::os::raw::c_void) {
            let _ = std::panic::catch_unwind(|| {
                if handle.is_null() {
                    return;
                }
                let mut guard = DRIVER_INSTANCE.lock().unwrap();
                if let Some(ref mut driver) = *guard {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    if let Ok(rt) = rt {
                        let _ = rt.block_on(driver.shutdown());
                    }
                }
            });
        }

        // ── destroy ────────────────────────────────────────────────────

        #[no_mangle]
        pub extern "C" fn macaca_driver_destroy(handle: *mut std::os::raw::c_void) {
            let _ = std::panic::catch_unwind(|| {
                if handle.is_null() {
                    return;
                }
                let mut guard = DRIVER_INSTANCE.lock().unwrap();
                *guard = None; // Drop the driver
            });
        }

        // ── free string ────────────────────────────────────────────────

        #[no_mangle]
        pub extern "C" fn macaca_driver_free_string(s: *mut std::os::raw::c_char) {
            let _ = std::panic::catch_unwind(|| {
                if !s.is_null() {
                    unsafe {
                        let _ = std::ffi::CString::from_raw(s);
                    }
                }
            });
        }
    };
}
