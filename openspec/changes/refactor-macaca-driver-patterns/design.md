## Context

`macaca-driver` sits after `macaca-tools` and before `macaca-web` / `macaca-framework` consumers. It is responsible for:

- static driver contract: `SoftwareDriver`
- dynamic plugin loading: `DynamicDriver`
- directory discovery: `DriverLoader`
- runtime registry: `DriverRegistry`
- dynamic tool proxying: `DynamicTool`

The current code keeps the ABI stable, but several design responsibilities are mixed:

- `DriverLoader::load_driver` validates manifests, resolves libraries, serializes config, and directly constructs `DynamicDriver`.
- `DynamicDriver::load` resolves ABI symbols, creates the handle, reads manifest JSON, and builds domain manifest.
- `DynamicTool::execute_streaming` owns command inputs, callback state, trace enrichment, and FFI execution.
- Trace enrichment logic is local to the streaming callback.

The design-pattern refactor plan for `macaca-driver` requires five slices: factory, command, trace adapter, dynamic proxy, and session state.

## Goals

- Introduce the five planned primitives in one coherent proposal.
- Keep each implementation slice additive and behavior-preserving.
- Give later upper-layer migrations stable canonical entrypoints.
- Make deprecated legacy entrypoints easy to find without deleting them.

## Non-Goals

- Do not rewrite the driver ABI.
- Do not change existing dynamic driver runtime behavior.
- Do not introduce new dependencies.
- Do not create application-specific or driver-specific paths.

## Decisions

### 1. Factory Pattern for driver creation

Add:

- `DriverCreateContext`
- `DriverFactory`
- `DynamicDriverFactory`

`DriverLoader::load_driver` will construct a `DynamicDriverFactory` and call `DriverFactory::create`. The old `DynamicDriver::load` remains as a deprecated compatibility wrapper.

### 2. Command Pattern for driver tool execution

Add:

- `DriverCommand`
- helpers for execute vs execute streaming

`DynamicTool` will convert incoming tool execution into `DriverCommand` before invoking the proxy. This makes future resume/status/driver-specific actions fit the same shape without hardcoding driver names.

### 3. Adapter Pattern for trace enrichment

Add:

- `DriverTraceAdapter`

The adapter enriches incoming `TraceEvent` with driver identity and timestamp when missing. This preserves current behavior and prevents future code from duplicating trace mutation logic.

### 4. Proxy Pattern for dynamic ABI calls

Add:

- `DynamicDriverProxy`
- `DynamicDriverSymbols`

The proxy owns the opaque handle and function pointer call helpers. `DynamicDriver` remains the public driver object and retains the loaded library field for drop-order safety.

### 5. State Pattern for execution session state

Add:

- `DriverSessionState`

Streaming callback data moves from an inline local struct into a named state object. The state remains scoped to the blocking FFI call and is not detached.

## Alternatives Considered

### Split `dynamic_driver.rs` first

This would reduce file size quickly, but it would touch unsafe ABI and drop ordering too broadly in one slice. It is deferred until the new primitives are in place.

### Trace-first refactor

This would target user-visible trace concerns quickly, but would leave construction and execution boundaries unchanged. It risks adding another bridge layer rather than reducing coupling.

## Migration Strategy

1. Add all OpenSpec artifacts for the five-slice scope.
2. Add `factory.rs` and migrate `DriverLoader` internals.
3. Add `command.rs` and route `DynamicTool` execution through commands.
4. Add `trace.rs` and route streaming callback enrichment through it.
5. Add `dynamic_proxy.rs` and route FFI execute/tool-definition/health/shutdown calls through it.
6. Add `session.rs` and route streaming callback state through it.
7. Mark legacy direct entrypoints as deprecated but keep compatibility.

## Verification

- `openspec validate refactor-macaca-driver-patterns --strict`
- `cargo test -p macaca-driver -- --nocapture`
- `cargo check -p macaca-driver`
- workspace `cargo check`
- deprecated-call containment grep for old driver entrypoints
- `gitnexus_detect_changes(scope: "all")`
