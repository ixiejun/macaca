---
name: macaca-dev-guide
description: Use when editing Macaca Agent OS Rust crates, frontend shells, OpenSpec changes, microkernel/service/module boundaries, WASM/app-runtime work, GenUI surfaces, autonomous task execution, LLM/driver/skill/MCP services, or Macaca OS architecture decisions.
---

# Macaca Agent OS Development Guide

## Prime Directive

Macaca is a generic Agent OS and autonomous application platform, not a single
application runner. Treat it like a microkernel operating system for agentic
software:

```text
Microkernel + Service Runtime + Application ABI + Autonomous Planner + Plugin/Module Ecosystem
```

Every change must preserve this model. Macaca should run 24/7, understand goals,
plan and re-plan work, delegate across agents and services, recover from
interruptions, and complete long-running tasks with minimal human instruction.
Humans provide goals, policy, approvals, and corrections; the platform owns
autonomous planning, execution, monitoring, traceability, and recovery.

Applications own product behavior and UI. The OS owns only generic contracts,
services, policy, trace, audit, scheduling, resource control, autonomy loops, and
package/runtime boundaries. Never hardcode workflow names, application names,
provider names, driver names, model names, chain names, or business-domain
branches into OS-layer code.

## Authoritative Documents

Read these before any architecture, behavior, interface, or non-trivial refactor:

- `openspec/AGENTS.md`: OpenSpec process and delta-spec format.
- `macaca/docs/macaca-os-architecture-governance.md`: stable architecture governance.
- `macaca/docs/macaca-os-microkernel-boundaries.md`: constitutional kernel/service/application/shell boundaries.
- `macaca/docs/macaca-os-serviceization-allowlist.md`: service admission and rejection rules.
- `macaca/docs/design_patterns.md`: approved design-pattern vocabulary.
- `docs/openharmony-microkernel-architecture-for-macaca-agent-os.md`: research background for the stable OS ecosystem vision and future boundaries.

The files under `references/` are secondary historical notes. If they conflict
with the documents above or current code, trust current code plus the stable
governance docs. Phase plans and refactor documents are historical evolution
evidence, not current implementation instructions.

## Current Workspace Shape

The real Rust workspace is `macaca/`. Crates are now grouped by layer:

```text
macaca/crates/foundation/    # proto, ipc, persist
macaca/crates/kernel/        # microkernel primitives only
macaca/crates/services/      # replaceable capability families
macaca/crates/runtime/       # runtime, framework, runtime-host providers
macaca/crates/application/   # app model and agent abstractions
macaca/crates/facade/        # SDK/SystemFacade clients
macaca/crates/shells/        # web and cli adapters
macaca/crates/tests/         # integration and boundary gates
frontend/                    # Next.js presentation shell
openspec/                    # proposed and baseline behavior contracts
```

Layer order is directional. Lower layers must not import upper layers:

```text
Applications
Application Framework / SDK
System Services
Service Runtime / Runtime Host
Microkernel Primitives
Foundation Protocols / IPC / Persistence Contracts
Host Environment
Presentation Shells
```

Presentation shells are adapters even when they temporarily host legacy
composition code.

## Stable Platform Vision

Build every feature toward this stable-state platform:

- Microkernel: small, typed, provider-neutral invariants for identity, policy,
  scheduling, resources, trace, audit, session/task primitives, registry, and
  service calls.
- Serviceized capabilities: LLM, Memory, Context, Task, Driver, Skill, MCP,
  Gateway, Application, Store, Entitlement, Payment, Web3, EVM, and UI/GenUI are
  replaceable service or module families.
- Autonomous operation: the platform can continuously plan, execute, observe,
  retry, recover, and report across long-running tasks without waiting for the
  user to micromanage each step.
- Application platform: YAML, WASM, GenUI, headless, paid, gateway, and optional
  Web3/EVM applications run through public ABI, manifest, capability, policy,
  package, and service boundaries.
- Low human intervention: approval, budget, safety, entitlement, and policy gates
  are explicit; routine execution should be self-directed and auditable.
- Generic extensibility: third-party applications, plugins, drivers, skills, MCP
  servers, gateways, and optional modules should install and run without Macaca
  source-code changes.

## Ownership Rules

### Microkernel May Own

- Identity for apps, agents, sessions, tasks, services, packages, developers, and tenants.
- Service and capability registries.
- Typed IPC/service-call facade and trace-required dispatch.
- Policy, resource, scheduler, session, task, package-guard primitives.
- Trace/audit buses and replayable evidence identifiers.

Kernel code must not construct concrete providers.

### System Services Must Own

LLM, Memory, Context, Task planning/execution/review, Driver, Skill, MCP,
Gateway, Application lifecycle, Store, Entitlement, Payment/A2A, Web3, EVM, and
third-party provider adapters. A built-in provider is still a service provider,
not kernel logic.

Every service needs a descriptor, lifecycle, health/snapshot command, typed
commands/results, structured unavailable/unsupported/denied/failure states,
trace context on every call, policy before side effects, sanitized audit, and
built-in/plugin/remote/mock/unavailable replacement.

Task, planner, execution-control, recovery, and review services are autonomy
services. They must be designed for continuous unattended operation: explicit
state machines, resumable checkpoints, bounded retries, policy-aware delegation,
observable progress, and clear escalation when human approval is truly required.

### Application Boundary

The application framework owns manifests, package metadata, ABI, lifecycle,
app-scoped capability declarations, permission declarations, WASM adapters,
YAML adapters, GenUI intent validation, and app-owned UI surface metadata.

Applications may orchestrate services, agents, tasks, MCP, skills, and UI only
through declared capabilities and service boundaries.

### Shell Boundary

`macaca-web`, `macaca-cli`, gateways, and `frontend/` may parse input, call
`SystemFacade` or focused clients, render state/GenUI/approval/trace/diagnostics,
and subscribe to events. They must not become semantic owners for task planning,
agent execution, payments, packages, drivers, skills, MCP, applications, chains,
or provider lifecycle.

## Mandatory Workflow

For business behavior, OS behavior, public interfaces, dependency ownership, or
non-trivial refactors:

1. Read current code and the relevant stable governance/research documents.
2. Use Superpowers brainstorming for options and risks.
3. Use Superpowers write-plan for the selected approach.
4. Create or update an OpenSpec change before implementation.
5. Run GitNexus impact analysis before editing symbols.
6. Implement in small, additive, reversible steps.
7. Keep old direct paths deprecated during compatibility periods when callers still exist.
8. Add or update tests and boundary gates.
9. Verify code, OpenSpec, trace, audit, and dependency boundaries agree.

Skipping this workflow is an architecture defect.

## Design Pattern Checklist

Before choosing an implementation, check whether one of these patterns fits:

- Facade: `SystemFacade`, focused SDK clients, service clients.
- Command: all cross-boundary operations are typed command/result DTOs.
- Adapter/Bridge: providers, transports, plugins, shells, WASM host imports.
- Strategy: provider choice, routing, policy, assignment, payment, chain adapters.
- Decorator: trace, policy, resource, entitlement, and metering around service calls.
- State: service, application, task, payment, package, and execution lifecycles.
- Observer: trace, audit, event log, task events, service events, SSE subscriptions.
- Memento: snapshots, checkpoints, replayable event/audit records.
- Specification: dependency gates, package admission, version constraints.
- Abstract Factory: approved composition roots for providers/modules.

Use patterns to clarify ownership and extension points; do not over-abstract.

## Serviceization Guardrails

Serviceization means ownership transfer, not file movement. A capability is not
serviceized until callers use a service/facade path with trace, policy,
structured errors, health/snapshot, and replacement mechanics.

Reject these changes:

- Kernel depends on concrete providers or presentation shells.
- SDK constructs providers, runtimes, database backends, wallets, or chain clients.
- Web/CLI/frontend define OS semantics for tasks, payments, packages, chains, drivers, skills, MCP, or app execution.
- Generic OS code branches on app/provider/model/driver/gateway/chain/payment names.
- Service calls lack trace, policy, or structured unavailable behavior.
- Autonomous execution loops hide state, depend on manual prompting, or cannot resume after restart.
- Optional modules crash, hang, silently fall back, or fake success when absent.
- Logs/traces/snapshots expose raw secrets, prompts, manifests, WASM bytes, package bytes, private keys, credentials, raw signatures, provider payloads, or unbounded output.

## Stable Capability Model

Use this as the current target model, not as a phased delivery checklist:

| Capability Area | Stable Ownership |
| --- | --- |
| Kernel | Invariants, identity, policy facade, registries, scheduling primitives, resource primitives, trace/audit primitives |
| Service Runtime | Service lifecycle, typed calls, decorators, health, snapshots, local/remote/plugin transport |
| SDK/SystemFacade | Stable clients, command DTOs, Null Object behavior, provider-neutral developer/shell boundary |
| Autonomy Services | Goal/task planning, execution control, delegation, review, recovery, retry, escalation |
| Intelligence Services | LLM, context composition, memory, retrieval, knowledge indexing, model/budget/rate policy |
| Capability Services | Driver, Skill, MCP, Gateway, Store, Entitlement, Payment/A2A, Web3, EVM |
| Application Framework | Manifest, ABI, package metadata, WASM/YAML adapters, lifecycle, permissions, app-owned UI/GenUI |
| Shells | Input adapters, rendering, approval surfaces, diagnostics, trace replay, GenUI mounting |

## Working Rules

- All new Rust code needs clear English comments for non-obvious behavior and
  operating principles.
- Keep Rust files under 500 lines. Split by ownership when files grow.
- Prefer current project patterns over new dependencies.
- Use provider-neutral DTOs in `macaca-proto` for shared contracts.
- Put service/provider wrappers and module bootstrapping in `macaca-runtime-host`.
- Put developer/shell-facing clients in `macaca-sdk`.
- Keep application-specific demos outside generic OS semantics.
- Application-owned UI is generic: host owns shell, policy, trace, and bridge;
  app owns center experience and business presentation.
- WASM guests call host capabilities through declared ABI/service boundaries;
  guests must not own secrets or bypass policy.
- Prefer autonomous default behavior: agents should proceed from goals, inspect
  evidence, use tools, verify progress, and escalate only for policy, safety, or
  genuinely ambiguous product decisions.

## Verification Commands

Run the narrowest useful set first, then broaden when touching shared behavior:

```bash
cd macaca
cargo check -p macaca-<crate>
cargo test -p macaca-<crate> <test_filter>
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests route_c_baseline
cargo check --workspace
```

Frontend shell changes:

```bash
cd frontend
npm run lint
npx tsc --noEmit
npx next dev --port 3000
```

Local stack restart for manual checks:

```bash
cd macaca && cargo run --bin macaca -- web
cd frontend && npx next dev --port 3000
```

Expected ports: frontend `3000`, Rust API `3001`. `GET /` on `3001` returning a
JSON 404 that says the API server does not host a web UI is normal.

## Debugging Order

When a runtime path fails, prove each boundary separately:

1. Contract exists in `macaca-proto`.
2. Provider/service is exported by its crate.
3. Provider is registered in `ServiceRuntime` or the approved composition root.
4. SDK/SystemFacade or focused client can call it.
5. Shell route only adapts input/output and emits/replays trace.
6. Application declares the required capability/service.
7. Unavailable/denied/unsupported states are structured and visible.
8. Trace/audit evidence is persisted before streaming to UI.
9. Autonomous loops can resume, retry, or escalate from the persisted state.

Do not stop at the first repaired subpath; separate "this boundary now works"
from remaining end-to-end blockers.

## OpenSpec Pointers

Use `openspec list`, `openspec list --specs`, and
`openspec validate <change-id> --strict`. Change IDs should be verb-led and
specific, for example `serviceize-agent-execution-v1`.

Behavior deltas belong under `openspec/changes/<change-id>/` with
`proposal.md`, `design.md`, `tasks.md`, and spec deltas. `openspec/specs/` is
the baseline fact source.
