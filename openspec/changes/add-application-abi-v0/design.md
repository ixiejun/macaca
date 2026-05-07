# Design: Application ABI v0

## Context

Route C has already established the required foundation for Application ABI v0:

- Phase 01 defines microkernel primitives such as identity, service ids, capability ids, policy requests, trace context, session/task primitives, and resource scopes.
- Phase 02 defines system service contracts and trace-required service commands.
- Phase 03 defines the local-first service bus boundary.
- Phase 04 defines Package Manifest v0 and runtime guard metadata for YAML and future WASM packages.

Phase 05 uses those boundaries to define the public application contract. Applications must not import `macaca-web`, grab `Arc<AppState>`, or call internal Rust services directly. They call the host through ABI imports. The host validates trace/policy boundaries and routes to system services or structured unavailable results.

## Goals

- Define Application ABI v0 as provider-neutral, serde-friendly protocol contracts.
- Represent ABI exports for `app:init`, `app:start`, `app:handle_event`, `app:render`, `app:pause`, `app:resume`, `app:shutdown`, and `app:upgrade`.
- Represent ABI imports for `macaca:capability/request`, `macaca:task/create_goal`, `macaca:task/query`, `macaca:trace/emit`, `macaca:ui/render`, `macaca:storage/get`, `macaca:storage/set`, `macaca:payment/create_intent`, and `macaca:service/call`.
- Provide an `ApplicationHost` facade that hides internal service wiring and enforces trace/policy-ready boundaries.
- Adapt existing YAML applications into ABI applications without breaking current YAML runtime behavior.
- Add a WASM loader stub that loads manifest/ABI declarations but returns structured `RuntimeUnavailable` on execution.
- Make ABI lifecycle, host calls, adapter selection, unavailable outcomes, and checkpoints traceable and auditable.
- Keep all new code well-commented in English and split into files under the project file-size limit.

## Non-Goals

- Do not execute WASM.
- Do not implement a full GenUI runtime.
- Do not implement actual payment settlement or Store entitlement checks.
- Do not make `macaca-web` the owner of ABI semantics.
- Do not make application ABI depend on concrete providers, app names, workflow names, driver names, gateway names, or chains.

## Superpowers Brainstorm Summary

### Current Problem

Macaca applications are still mainly represented as YAML configuration and are loaded through internal Rust paths. That works for current demos but does not provide a stable public ABI for third-party applications, future WASM packages, SDK authors, Store distribution, or audit tooling.

### Why Phase 05 Must Solve It

Package Manifest v0 can describe software, but packages still need a runtime contract. Before GenUI, plugin runtime, Store, entitlement, paid applications, and optional Web3/EVM modules can be built cleanly, the OS needs a narrow application boundary that every runtime kind can target.

### Options Considered

1. **Protocol-first ABI with host facade and adapters.**
   - Pros: stable public boundary, preserves YAML apps, keeps internal services hidden, supports future WASM without pretending execution exists now.
   - Cons: requires parallel adapter/facade code before all consumers migrate.
   - Verdict: recommended.

2. **Extend the existing YAML manifest only.**
   - Pros: smaller short-term patch.
   - Cons: keeps applications configuration-shaped rather than ABI-shaped, weak support for lifecycle/import/export semantics, poor fit for WASM and Store.
   - Verdict: rejected as too narrow for Route C.

3. **Implement real WASM runtime now.**
   - Pros: gives an executable ABI immediately.
   - Cons: combines ABI, sandbox, WASI, component model, security, package guard, and host services in one risky step.
   - Verdict: rejected as over-scoped for Phase 05.

4. **Let applications call internal Rust facades directly.**
   - Pros: faster for current in-repo apps.
   - Cons: breaks microkernel boundaries, blocks third-party ABI compatibility, makes trace/policy audit inconsistent.
   - Verdict: rejected.

## Recommended Plan

Implement the ABI additively. First define protocol types, then add lifecycle and host facade contracts, then adapt YAML apps, then add the WASM metadata loader stub. Migrate only the minimal safe integration points needed to prove task, trace, and storage host paths work through the facade. Leave broad consumer migration for later phases.

## Design Patterns

- **Facade**: `ApplicationHost` is the only SDK/application-facing way to request system capabilities, task operations, trace emission, UI rendering, storage, payment intents, and service calls.
- **Adapter**: YAML applications and future WASM packages both adapt into the same `ApplicationAbiInstance` contract.
- **Command**: each host import is represented as a typed host command with input, trace context, metadata, and structured result.
- **State**: application lifecycle is modeled as explicit state transitions; invalid transitions return structured errors.
- **Memento**: pause/resume/upgrade flows can carry checkpoint payloads without exposing internal runtime objects.
- **Specification**: ABI declaration, permissions, lifecycle transition, and host import availability are validated by composable rules instead of app-specific branching.
- **Observer**: lifecycle events, host calls, adapter decisions, and unavailable outcomes emit trace/audit/log events.
- **Null Object**: unimplemented optional imports such as payment or WASM execution return structured unavailable values instead of panics or fake success.

## Contract Shape

### `macaca-proto/src/application_abi.rs`

The protocol module should contain data-only types:

- `ApplicationAbiVersion`
- `ApplicationExport`
- `ApplicationImport`
- `ApplicationAbiDeclaration`
- `ApplicationLifecycleState`
- `ApplicationLifecycleTransition`
- `ApplicationEvent`
- `ApplicationRenderRequest`
- `ApplicationRenderResult`
- `ApplicationHostImport`
- `ApplicationHostCommand`
- `ApplicationHostCommandResult`
- `ApplicationCheckpoint`
- `ApplicationAbiError`

ABI imports and exports should be typed value objects or enums with `Custom(String)` variants where useful. This keeps future imports, exports, and host commands extensible without kernel source edits.

### `macaca-app/src/abi.rs`

The Application Framework ABI module should define the runtime-facing traits and conversion contracts:

- `ApplicationAbiInstance`
- `ApplicationAbiAdapter`
- `ApplicationAbiDescriptor`
- `ApplicationAbiLoadResult`
- `ApplicationAbiRuntimeUnavailable`

These contracts should not expose internal web state, framework runner state, or concrete provider clients.

### `macaca-app/src/lifecycle.rs`

The lifecycle module should implement a small state machine:

```text
Declared -> Initialized -> Started -> Paused -> Resumed -> ShuttingDown -> Stopped
Declared -> Initialized -> Failed
Started -> Failed
Paused -> Failed
```

The final implementation may refine names, but lifecycle transitions must be explicit, testable, and traceable. Invalid transitions must return structured errors.

### `macaca-app/src/host.rs`

`ApplicationHost` should expose controlled host imports:

- capability request
- task create goal
- task query
- trace emit
- UI render request
- app-scoped storage get/set
- payment create intent
- generic service call

The facade should require trace context for host calls where Route C governance requires trace. It should route implemented paths through existing task/trace/storage primitives and return `Unavailable`, `DisabledByPolicy`, or `RuntimeUnavailable` where the backing system does not exist yet.

### `macaca-sdk/src/application.rs`

The SDK module should provide developer-facing builders/helpers for ABI declarations and host commands. It should not require application authors to import `macaca-web`, `macaca-framework`, `macaca-kernel`, or internal host state.

## YAML Compatibility Adapter

Existing YAML applications remain first-class. The YAML adapter should:

- read existing app/package metadata;
- map app id, name, version, entry agent, workflow references, capabilities, allowed tools, and package runtime data into an ABI descriptor;
- emit lifecycle events for initialization/start boundaries;
- avoid hardcoded app names or workflow routing;
- leave existing YAML execution behavior intact until later migrations explicitly move it behind ABI dispatch.

## WASM Loader Stub

The WASM stub should load package metadata and ABI declarations only. If execution is requested, it must return structured `RuntimeUnavailable` and emit trace/log records explaining that WASM execution is intentionally not available in Phase 05. It must not attempt to instantiate or execute WASM bytes.

## Trace, Audit, And Logging

Phase 05 implementation must log and trace:

- ABI declaration parsed;
- ABI adapter selected;
- lifecycle transition requested;
- lifecycle transition accepted or rejected;
- host import command received;
- policy/permission boundary evaluated or marked pending;
- task create/query call routed;
- trace emit routed;
- storage get/set routed;
- UI/payment/service call unavailable or routed;
- checkpoint created/restored;
- WASM runtime unavailable.

Trace/log payloads should include application id, package id when available, ABI version, lifecycle state, host import name, command id, session id, trace id, structured status, and structured error code. They must not include secrets, provider credentials, private keys, raw encrypted package contents, or payment credentials.

## Compatibility And Regression

The change must preserve:

- `RC-APP-001`: YAML application loading.
- `RC-CHAT-001`: `/api/chat/v2` creates a session.
- `RC-GOAL-001`: goal -> planner -> task -> worker -> review -> coordinator resume remains functional.

Current YAML application paths may continue to run outside ABI dispatch while the compatibility adapter is introduced. The acceptance requirement is that ABI metadata and lifecycle boundaries can represent those apps without regressing existing behavior.

## Risks / Trade-offs

- **Risk: ABI becomes too broad or Store-heavy.** Mitigation: keep Store, entitlement, real payment settlement, GenUI runtime, Web3, and EVM as explicit non-goals.
- **Risk: Facade hides policy gaps.** Mitigation: host calls must carry trace context and structured policy-ready metadata even when a real policy backend is deferred.
- **Risk: YAML adapter becomes app-specific.** Mitigation: adapter maps manifest fields generically and hardcode scans reject demo/business names.
- **Risk: WASM stub is mistaken for runtime support.** Mitigation: metadata load succeeds, execution returns structured `RuntimeUnavailable` with logs/trace.
- **Risk: new files become giant modules.** Mitigation: split proto, ABI traits, host facade, lifecycle state machine, SDK helpers, and tests into focused files under 500 lines.

## Open Questions

- None blocking Phase 05. Real WASM execution, GenUI rendering, Store/entitlement, and payment settlement belong to later Route C phases.
