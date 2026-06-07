## Context

`macaca-cli` is intentionally refactored after lower-level crates because it depends on `macaca-web`, `macaca-kernel`, `macaca-gateway`, `macaca-app`, `macaca-llm`, `macaca-tools`, and shared proto types.

The current implementation is small enough to review, but responsibilities are mixed:

- `main.rs` parses Clap commands, initializes logging, dispatches command behavior, manages systemd integration, and starts the web server.
- `commands.rs` exposes command-like functions and also owns kernel construction helpers and stub LLM provider behavior.
- Repeated command functions rebuild config, stub LLM, tools, and kernel independently.

## Goals

- Introduce a Command-pattern handler boundary as the canonical dispatch path.
- Preserve current CLI behavior and help output.
- Keep old exported command functions available but deprecated for migration lookup.
- Avoid adding dependencies or application-specific behavior.
- Keep the first implementation slice limited to CLI dispatch and tests.

## Non-Goals

- Do not remove old command functions.
- Do not change CLI subcommand names, flags, help text, stdout formatting, or exit semantics.
- Do not replace the stub LLM provider.
- Do not redesign logging in this change.
- Do not introduce `CliRuntimeContext` in this change; it is a follow-up slice after handlers exist.
- Do not move systemd behavior behind a new facade yet.

## Design Decisions

### Command Handlers

Add a `CliCommandHandler` trait implemented by focused handlers:

- `RunCommandHandler`
- `AgentsCommandHandler`
- `StatusCommandHandler`
- `VersionCommandHandler`
- `WebCommandHandler`

The canonical dispatch path should call handlers directly. The handlers may initially reuse private shared helpers from `commands.rs` to preserve behavior.

### Deprecated Compatibility Functions

Existing exported functions should remain available:

- `run_kernel`
- `list_agents`
- `show_status`
- `create_kernel`

When a handler replaces an exported function as the canonical call path, mark the old function with `#[deprecated]` and make it delegate to the handler or shared implementation. This keeps migration discovery possible without deleting APIs.

New dispatch code and migrated tests should avoid calling deprecated wrappers. If a compatibility test intentionally calls a deprecated function, the allowance must be local and explicit.

### Systemd Web Lifecycle

The web command currently notifies systemd readiness and spawns a watchdog heartbeat before serving web. The first handler slice must preserve that behavior in the `web` path without changing timing.

### Future Runtime Context

`CliRuntimeContext` is intentionally deferred. Introducing it together with handlers would increase the blast radius because it changes config-loading ownership and command signatures. Once handler boundaries exist, a later change can pass loaded config and runtime inputs through a context.

## Risks

- CLI behavior is user-visible even when call-graph risk is low.
- Deprecated exported functions can produce warnings in tests or internal callers if migration is incomplete.
- Moving systemd behavior can change service startup timing.
- Reworking command internals can accidentally alter `run_kernel` signal behavior or gateway startup order.

## Validation

- `openspec validate refactor-cli-command-handlers --strict`
- `cargo fmt --all`
- `cargo check -p macaca-cli`
- `cargo test -p macaca-cli --lib`
- `cargo run -p macaca-cli -- --help`
- `cargo run -p macaca-cli -- web --help`
- GitNexus impact before edits and detect-changes before finishing.
