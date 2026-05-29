//! Application-owned WASM guest for the Codex WASM Workbench.
//!
//! The guest deliberately does not implement Macaca OS semantics. It only
//! exposes deterministic application exports and static command templates that
//! a Macaca WASM host can translate into provider-neutral `service.call`,
//! `trace.emit`, and `ui.render` imports. The operating principle is the
//! Facade pattern: this component presents application intent while all policy,
//! filesystem, process, sandbox, Git, review, diagnostics, trace, and audit
//! behavior remains in Macaca services.

// WASM guests need stable exported symbol names so the host can discover
// lifecycle functions. The crate still uses no `unsafe` blocks; the exported
// ABI surface is limited to pointer/length pairs and integer status codes.

/// Stable status returned when the app export accepted the host call.
const STATUS_ACCEPTED: i32 = 0;

/// Stable status returned when a host passes an empty trace identifier.
const STATUS_MISSING_TRACE: i32 = 2;

/// Return a bounded status code for an application lifecycle export.
///
/// Macaca requires every cross-boundary command to carry trace context. The
/// guest mirrors that invariant by rejecting empty trace pointers. It does not
/// inspect provider names, application names, model names, or business data.
fn traced_status(trace_ptr: *const u8, trace_len: usize) -> i32 {
    if trace_ptr.is_null() || trace_len == 0 {
        return STATUS_MISSING_TRACE;
    }
    STATUS_ACCEPTED
}

/// Initialize application-owned runtime state.
///
/// The component is intentionally stateless in this first package. Durable
/// Thread/Turn/Item state belongs to `service.interaction`, so initialization
/// only validates that the host supplied a trace reference.
#[no_mangle]
pub extern "C" fn app_init(trace_ptr: *const u8, trace_len: usize) -> i32 {
    traced_status(trace_ptr, trace_len)
}

/// Start the application session.
///
/// The host is expected to subscribe to app-protocol events and create the
/// initial thread/turn through services. This export only acknowledges the
/// lifecycle transition at the application boundary.
#[no_mangle]
pub extern "C" fn app_start(trace_ptr: *const u8, trace_len: usize) -> i32 {
    traced_status(trace_ptr, trace_len)
}

/// Render the application-owned workspace surface.
///
/// Real rendering is performed by the host UI bridge using `ui.render`. The
/// guest export remains a bounded trigger so raw UI payloads are not embedded
/// in runtime logs or snapshots.
#[no_mangle]
pub extern "C" fn app_render(trace_ptr: *const u8, trace_len: usize) -> i32 {
    traced_status(trace_ptr, trace_len)
}

/// Handle an application event from the host.
///
/// Event payloads should stay bounded and sanitized before crossing into a
/// guest. This export accepts a trace and event slice but intentionally does not
/// parse the event; routing and event ownership remain in `service.app_protocol`.
#[no_mangle]
pub extern "C" fn app_handle_event(
    trace_ptr: *const u8,
    trace_len: usize,
    event_ptr: *const u8,
    event_len: usize,
) -> i32 {
    let trace_status = traced_status(trace_ptr, trace_len);
    if trace_status != STATUS_ACCEPTED {
        return trace_status;
    }
    if event_ptr.is_null() || event_len == 0 {
        return STATUS_MISSING_TRACE;
    }
    STATUS_ACCEPTED
}

/// Return a pointer to a static, sanitized workflow declaration.
///
/// Hosts can read this pointer only in test harnesses that know the static
/// memory layout. The string is intentionally bounded and contains only service
/// identifiers, command names, and UI surface ids. It never includes prompts,
/// secrets, file contents, provider payloads, or application-specific OS
/// routing branches.
#[no_mangle]
pub extern "C" fn codex_workbench_plan_ptr() -> *const u8 {
    CODEX_WORKBENCH_PLAN.as_ptr()
}

/// Return the byte length of the static workflow declaration.
#[no_mangle]
pub extern "C" fn codex_workbench_plan_len() -> usize {
    CODEX_WORKBENCH_PLAN.len()
}

/// Bounded command sequence that demonstrates the application-owned coding
/// workflow. Each step names the owning service. The host decides availability,
/// policy, execution, audit, and degradation through Macaca service contracts.
const CODEX_WORKBENCH_PLAN: &str = r#"{
  "schema": "macaca.codex_wasm_workbench.plan.v1",
  "steps": [
    {"service": "service.interaction", "command": "interaction.thread.start"},
    {"service": "service.interaction", "command": "interaction.turn.start"},
    {"service": "service.app_protocol", "command": "app_protocol.subscription.create"},
    {"service": "service.file", "command": "file.read"},
    {"service": "service.code_intelligence", "command": "code.search"},
    {"service": "service.approval", "command": "approval.request.create"},
    {"service": "service.hook", "command": "hook.pre_tool.run"},
    {"service": "service.git", "command": "git.apply_patch"},
    {"service": "service.sandbox", "command": "sandbox.environment.prepare"},
    {"service": "service.process", "command": "process.exec"},
    {"service": "service.review", "command": "review.start"},
    {"service": "service.diagnostics", "command": "diagnostics.snapshot"},
    {"service": "service.interaction", "command": "interaction.turn.complete"}
  ],
  "ui_surface": "main"
}"#;
