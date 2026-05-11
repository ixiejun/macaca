## 1. Preparation

- [x] 1.1 Read the plugin enrichment plan and Hook Bus design.
- [x] 1.2 Read framework, task, context, memory, gateway, LLM, approval, and session lifecycle integration points.
- [x] 1.3 Run GitNexus impact before editing existing symbols and report blast radius.

## 2. Protocol Contracts

- [x] 2.1 Add hook name, kind, descriptor, invocation, context, result, timeout policy, and failure policy DTOs.
- [x] 2.2 Add schema-validated result structures for observer, mutating, blocking, and approval hooks.
- [x] 2.3 Add detailed English comments explaining hook safety and bounded payload rules.

## 3. Runtime Hook Bus

- [x] 3.1 Add `PluginHookRegistry`.
- [x] 3.2 Add `PluginHookBus`.
- [x] 3.3 Add `PluginHookRunner` with priority ordering.
- [x] 3.4 Add timeout and failure policy strategies.
- [x] 3.5 Add trace/audit event emission for hook invocation.
- [x] 3.6 Add structured no-op/unavailable behavior for missing handlers.

## 4. Integration Points

- [x] 4.1 Integrate agent/application lifecycle hooks.
- [x] 4.2 Integrate prompt/context build hooks.
- [x] 4.3 Integrate tool call hooks.
- [x] 4.4 Integrate LLM call hooks.
- [x] 4.5 Integrate memory ingest hooks.
- [x] 4.6 Integrate gateway message hooks.
- [x] 4.7 Integrate approval lifecycle hooks.
- [x] 4.8 Integrate session/task lifecycle hooks.

## 5. Verification

- [x] 5.1 Run `openspec validate add-plugin-hook-bus-v1 --strict`.
- [x] 5.2 Run `cargo fmt --all --check`.
- [x] 5.3 Run `cargo check --workspace`.
- [x] 5.4 Run `cargo test -p macaca-proto plugin_hook`.
- [x] 5.5 Run `cargo test -p macaca-runtime-host plugin_hook`.
- [x] 5.6 Run `cargo test -p macaca-framework plugin_hook`.
- [x] 5.7 Run `cargo test -p macaca-integration-tests plugin_hook`.
- [x] 5.8 Run `npx gitnexus detect-changes -r agent` before commit.

Note: `npx gitnexus detect-changes -r agent` returned exit code 0, but GitNexus also reported read-only shadow-page/FTS replay errors while resolving file-symbol metadata. Treat the command as attempted, with index-side scope reporting degraded until the GitNexus database is reopened or rebuilt in read-write mode.
