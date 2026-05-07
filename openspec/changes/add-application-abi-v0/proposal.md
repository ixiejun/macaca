# Change: Add Application ABI v0

## Why

Route C Phase 05 must turn Macaca applications from YAML-only configuration into a stable Application ABI contract. This ABI is the public boundary third-party software will target when running on Macaca OS, while existing YAML applications must continue to run unchanged.

Without an explicit ABI, application lifecycle, capability calls, task creation, trace emission, UI rendering, app storage, payment intents, and generic service calls remain tied to internal Rust crates or presentation-layer shortcuts. That would violate the microkernel boundary and make future WASM, Store, entitlement, GenUI, plugin, and optional-module work harder to audit.

## What Changes

- Add Application ABI v0 protocol contracts in `macaca-proto` for lifecycle exports, host imports, application events, host commands, structured results, ABI errors, trace context, and checkpoint/state payloads.
- Add Application Framework ABI modules in `macaca-app` that expose a stable `ApplicationHost` facade, lifecycle state machine, YAML compatibility adapter, and non-executing WASM loader stub.
- Add SDK-facing application ABI helpers in `macaca-sdk` so application authors can build ABI requests without importing internal runtime crates.
- Preserve existing YAML applications by adapting current YAML manifests into ABI applications instead of replacing the current loader path in one step.
- Route `ApplicationHost` task, trace, and storage calls through existing system paths where safe, and return structured unavailable errors for imports that are declared but not yet backed by a real service.
- Emit structured logs and trace/audit records for ABI lifecycle transitions, host import calls, policy/permission boundaries, adapter selection, runtime unavailable outcomes, and checkpoint decisions.
- Require detailed English comments in all new Rust code explaining ABI boundaries, lifecycle operation, host command routing, trace/audit behavior, adapter behavior, and explicit non-goals.

## Impact

- Affected specs: `application-abi-v0`
- Affected crates: `macaca-proto`, `macaca-app`, `macaca-sdk`, with integration touchpoints in `macaca-framework` and `macaca-web`
- Affected tests: ABI serde/state-machine tests in `macaca-proto` and `macaca-app`, SDK application helper tests, YAML adapter compatibility tests, targeted web/framework compile checks
- Regression matrix references: `RC-APP-001`, `RC-CHAT-001`, `RC-GOAL-001`

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: Application Framework owns manifest, package metadata, WASM ABI, YAML compatibility, lifecycle, app storage, capability request, and permission declaration; applications cannot directly access `Arc<AppState>` or internal Rust runtime state.
- Follows `macaca/docs/route-c-regression-matrix.md`: Phase 05 must preserve YAML application loading, `/api/chat/v2` session creation, and goal pipeline behavior.
- Follows `macaca/docs/route-c-phase-template.md`: Superpowers brainstorm, OpenSpec proposal/design/tasks/spec, GitNexus impact before symbol edits, additive-first implementation, targeted tests, integration smoke, detect_changes before commit.
- Follows `macaca/docs/route-c-architecture-governance.md`: ABI lifecycle, service calls, task calls, trace calls, storage calls, UI calls, and payment calls must be traceable, policy-ready, structured on failure, and free of app/provider/driver/gateway/chain hardcoding.

## Non-Goals

- Do not implement a real WASM runtime, WASI host, component linker, sandbox, or bytecode execution in Phase 05.
- Do not implement GenUI rendering semantics beyond ABI declaration and host import boundaries.
- Do not implement Store, entitlement, payment provider settlement, subscription billing, Web3, or EVM execution.
- Do not migrate all web, framework, CLI, plugin, skill, MCP, driver, or runtime consumers to ABI in this phase.
- Do not delete existing YAML loader APIs; if direct paths become unsafe, mark them deprecated and keep them searchable for later migration.
- Do not hardcode application names, workflow names, provider names, driver names, gateway names, chain names, model names, or business-specific routing.
