# Macaca OS Architecture Governance

## Purpose

This document defines stable-state architecture governance for Macaca Agent OS. Macaca OS has established its boundaries around a microkernel, serviceized capabilities, and modular extension points; every future OS-layer, service-layer, application-framework-layer, and shell-layer change must preserve those boundaries.

## North Star

```text
Microkernel + Service Runtime + Application ABI + Plugin/Module Ecosystem
```

The system must support YAML applications, WASM applications, GenUI applications, headless applications, paid applications, gateway applications, and optional Web3/EVM applications without modifying OS source code for each business domain.

## Layer Order

Dependencies must point downward, or cross boundaries through facades:

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

Presentation shells are adapters, not semantic owners. Optional modules enter through service or module contracts and must not become base OS dependencies.

## Stable Ownership

Every new capability must answer "which layer owns this?" before implementation:

1. The microkernel owns only system invariants such as identity, policy, scheduling, resources, tracing, auditing, registration, and service calls.
2. System services own replaceable capability families and expose them through typed commands, descriptors, health checks, and structured errors.
3. The application framework owns application manifests, application ABI, application lifecycle, app-scoped permissions, and app-owned UI surfaces.
4. Plugins and optional modules extend OS capabilities, but must not become reverse dependencies of the base OS.
5. Web, CLI, gateways, and frontends only parse input, render state, handle approvals, show diagnostics, and subscribe to events.

Code that cannot be clearly assigned to these layers must not be merged.

## Required Design Patterns

Use the following patterns explicitly and with restraint:

- Facade: `SystemFacade`, focused SDK clients, service clients.
- Command: every cross-boundary operation must be a typed command/result.
- Adapter and Bridge: providers, transports, plugins, and shells adapt through contracts.
- Strategy: provider choice, policy, routing, assignment, payment, and chain adapters must be replaceable.
- Decorator: service calls receive trace, policy, resource, entitlement, and metering behavior at the boundary.
- State: application, task, service, payment, and package lifecycles must be modeled explicitly.
- Observer: trace, audit, event log, task events, and service events must be subscribable.
- Memento: snapshots, checkpoints, and audit records must be replayable.
- Specification: dependency gates, admission checks, and package version constraints must be executable.
- Abstract Factory: provider factories and module bootstrapping must stay in approved composition roots.

Do not invent abstract abstractions. Every abstraction must serve at least one current consumer and one clear future extension point.

## Mandatory Workflow

Any OS-layer behavior, interface, dependency, or ownership change must:

1. Read the current code and related documents.
2. Check active OpenSpec changes and existing specs.
3. Run GitNexus impact analysis before editing symbols.
4. Create or update OpenSpec proposal, design, tasks, and specs.
5. Update boundary documents, architecture governance, or the serviceization admission list when ownership changes.
6. Preserve public contracts; breaking changes must be expressed through version boundaries.
7. Run targeted tests, dependency-boundary tests, and audit replay checks.
8. Verify that logs, snapshots, and diagnostics do not leak sensitive data.

Skipping this process is an architecture defect.

## Acceptance Gates

Every OS-layer change must prove:

- Existing YAML, WASM, and GenUI applications still run.
- `/api/chat/v2` session creation and recovery do not regress.
- Task boards remain isolated by session scope.
- Trace and audit evidence remain replayable after refresh.
- Driver, Skill, MCP, LLM, Memory, Context, Application, Store, Payment, Web3, and EVM failures are structured unavailable/denied states, not crashes.
- Optional modules may be absent.
- No application-name or provider-name hardcoding appears below the application layer.
- Logs and snapshots are sanitized.

## Service Rules

Every system service must:

- Own a descriptor, lifecycle, health check, snapshot, and command surface.
- Require trace context on every call.
- Pass policy before side effects.
- Emit sanitized trace and audit events.
- Return structured unavailable, unsupported, denied, and failure states.
- Support built-in, plugin, remote, mock, and unavailable provider replacement.

Services must not import presentation shells and must not own application-specific business logic.

## SDK/SystemFacade Rules

The SDK is the stable developer-facing and shell-facing facade:

- It may define provider-neutral clients, commands, results, and Null Object behavior.
- It must not construct runtime-host providers, Web state, CLI state, app runtimes, driver runtimes, skill runtimes, MCP runtimes, payment providers, wallets, chain clients, or database backends.
- Missing services must return explicit unavailable behavior.

## Runtime Host Rules

`macaca-runtime-host` owns host-side service provider wrappers, module bootstrapping, plugin/runtime adapters, WASM host imports, service decorators, and sanitized diagnostics.

It must not own business workflows, raw presentation state, or hardcoded application behavior.

## Application Rules

The application framework owns manifests, ABI, lifecycle, app-scoped metadata, and version contracts. Applications may dynamically orchestrate services, especially WASM applications, but all orchestration must pass through declared capabilities and service boundaries.

YAML workflows are first-class application adapters, not kernel features. WASM flexibility is a runtime feature, not permission to bypass policy.

## Shell Rules

Web, frontend, CLI, and gateways may only:

- Parse input.
- Call `SystemFacade` or focused clients.
- Render state, GenUI, approval, trace, and diagnostics.
- Subscribe to events.

They must not become permanent semantic owners for planners, workers, payments, packages, drivers, skills, MCP, chains, or application lifecycles.

## Security And Audit Rules

- Trace must exist.
- Policy must run.
- Capability declarations must exist.
- Resource and entitlement checks must run before privileged side effects.
- Optional module absence must be explicit.
- Logs and snapshots must be bounded and sanitized.
- Raw secrets, prompts, manifests, WASM bytes, package bytes, private keys, credentials, raw signatures, raw provider payloads, and unbounded output must not enter observability surfaces.

## Change Review Questions

Before merging any OS-layer change, answer:

1. Which layer owns this behavior?
2. Is it a kernel invariant or a replaceable capability?
3. Which service or facade boundary carries the call?
4. What are the unavailable and Null Object behaviors?
5. Which trace and audit evidence proves the chain?
6. Which policy and capability gates protect it?
7. Which test or executable gate proves the boundary was not broken?
8. Which regression scenario catches system-contract breakage?

If these answers are unclear, the change is not ready.
