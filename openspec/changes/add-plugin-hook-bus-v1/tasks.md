## 1. Preparation

- [ ] 1.1 Read the plugin enrichment plan and Hook Bus design.
- [ ] 1.2 Read framework, task, context, memory, gateway, LLM, approval, and session lifecycle integration points.
- [ ] 1.3 Run GitNexus impact before editing existing symbols and report blast radius.

## 2. Protocol Contracts

- [ ] 2.1 Add hook name, kind, descriptor, invocation, context, result, timeout policy, and failure policy DTOs.
- [ ] 2.2 Add schema-validated result structures for observer, mutating, blocking, and approval hooks.
- [ ] 2.3 Add detailed English comments explaining hook safety and bounded payload rules.

## 3. Runtime Hook Bus

- [ ] 3.1 Add `PluginHookRegistry`.
- [ ] 3.2 Add `PluginHookBus`.
- [ ] 3.3 Add `PluginHookRunner` with priority ordering.
- [ ] 3.4 Add timeout and failure policy strategies.
- [ ] 3.5 Add trace/audit event emission for hook invocation.
- [ ] 3.6 Add structured no-op/unavailable behavior for missing handlers.

## 4. Integration Points

- [ ] 4.1 Integrate agent/application lifecycle hooks.
- [ ] 4.2 Integrate prompt/context build hooks.
- [ ] 4.3 Integrate tool call hooks.
- [ ] 4.4 Integrate LLM call hooks.
- [ ] 4.5 Integrate memory ingest hooks.
- [ ] 4.6 Integrate gateway message hooks.
- [ ] 4.7 Integrate approval lifecycle hooks.
- [ ] 4.8 Integrate session/task lifecycle hooks.

## 5. Verification

- [ ] 5.1 Run `openspec validate add-plugin-hook-bus-v1 --strict`.
- [ ] 5.2 Run `cargo fmt --all --check`.
- [ ] 5.3 Run `cargo check --workspace`.
- [ ] 5.4 Run `cargo test -p macaca-proto plugin_hook`.
- [ ] 5.5 Run `cargo test -p macaca-runtime-host plugin_hook`.
- [ ] 5.6 Run `cargo test -p macaca-framework plugin_hook`.
- [ ] 5.7 Run `cargo test -p macaca-integration-tests plugin_hook`.
- [ ] 5.8 Run `npx gitnexus detect-changes -r agent` before commit.
