# Macaca OS Microkernel Boundaries

## Purpose

This document defines the constitutional boundaries of Macaca Agent OS. Macaca OS is a microkernel Agent OS: the kernel owns only system invariants, while every replaceable capability must live as a system service, plugin, application-framework capability, or optional module.

When implementation convenience conflicts with this document, this document wins.

## First Principles

```text
The kernel guards system invariants.
Services own replaceable capabilities.
Applications own product behavior and UI.
Plugins extend the capability surface.
Store and entitlement own distribution and authorization.
Optional modules may be absent.
Web and CLI are shells.
```

The kernel must stay small, stable, typed, auditable, and provider-neutral. A capability being important does not mean it belongs in the kernel.

## What The Kernel May Own

Only system invariants and foundational coordination primitives may enter the microkernel:

- Identity: application, agent, session, task, service, capability, package, developer, tenant.
- Service registry: register, discover, check, and track service lifecycles.
- Capability registry: name and discover capabilities, without implementing them.
- IPC and service-call facade: typed command routing, transport abstraction, trace-required dispatch.
- Policy facade: abstract decisions for permission, budget, resource, region, entitlement, and approval.
- Trace and audit bus: append-only system evidence and replayable execution chains.
- Scheduler primitive: fairness and wakeup semantics, without business workflow.
- Resource manager facade: declare, reserve, meter, and release host resources.
- Session primitive: lifecycle, pause/resume, checkpoint identity, cancellation.
- Task primitive: state contracts for goals, tasks, and reviews, without planner implementation.
- Package runtime guard: signature, version constraint, permission, and entitlement admission before execution.

Kernel code must not construct concrete providers for these concepts.

## What The Kernel Must Not Own

The following capabilities are non-kernel by nature:

- LLM providers, model routing, prompts, pricing tables.
- Planners, reviewers, worker-loop implementations, task decomposition.
- Memory backends, vector stores, context engines, retrieval strategies.
- Driver runtimes, browser/IDE/desktop automation implementations.
- Skill runtimes, skill package loaders, encrypted skill execution.
- MCP protocol runtimes, tool attachment, external resource transport.
- Gateway adapters such as Slack, Telegram, Discord, Email, Feishu, DingTalk.
- Application manifest interpretation, YAML workflows, WASM runtime, GenUI rendering, app-owned UI.
- Store, entitlement, license, metering, package marketplace.
- Payment, A2A quotes, receipts, settlement, remote agent commerce.
- Web3, wallets, chain RPC, EVM, DApp execution.
- Web, CLI, frontend, approval UI, dashboards, trace viewers.

These capabilities may have built-in implementations, but a built-in implementation is still a service provider, not a kernel responsibility.

## Service Boundary

A system service owns one replaceable capability family. Every service must expose:

- A stable `service_id`, command names, descriptor, health state, and lifecycle.
- Typed command/result DTOs in a provider-neutral crate.
- Trace-required calls, structured errors, sanitized logs, and snapshots.
- Policy, resource, entitlement, and metering hooks before side effects.
- Null Object or unavailable behavior when a provider is absent.

Service providers may be built-in, plugin-backed, remote, mock, or unavailable. Consumers must not care where the implementation comes from.

## Application Boundary

Applications own product behavior and presentation intent. The application framework owns:

- Manifest and package metadata.
- YAML application adapter.
- WASM Application ABI.
- Application lifecycle and session envelope.
- App-scoped capability requests and permission declarations.
- GenUI intent validation and app-owned UI surface metadata.

Applications may orchestrate services, agents, tasks, MCP, skills, and UI, but only through declared capabilities and service boundaries.

## Plugin Boundary

Plugins extend the OS capability surface. A plugin must declare its manifest, capabilities, permissions, resources, lifecycle, service descriptors, and trace schema. Plugins must enter through the service/plugin registry and must not bypass kernel policy or the service runtime.

## Optional Module Boundary

Optional modules must never become required dependencies of the base OS. When Web3, EVM, a specific gateway, a specific driver, a paid package, or a provider-specific service is absent, the system must return a structured unavailable, disabled, or denied state.

Absence is a valid state. Crash, hang, silent fallback, and fake success are not valid states.

## Shell Boundary

`macaca-web`, frontend, CLI, and gateway entrypoints are thin shells:

- Receive user or external input.
- Convert it into SDK/`SystemFacade` commands.
- Render output, trace, GenUI, approval, and diagnostics.
- Subscribe to service events.

Shells must not define system semantics for tasks, sessions, payments, packages, drivers, skills, MCP, applications, or chains.

Task planning and review flow are autonomy-service responsibilities. Web/CLI
shells may submit typed service commands, subscribe to task events, and render
Task Board state, but decomposition strategies, fallback planning, review retry,
and terminal state repair must live behind Task/Autonomy service boundaries.

## Prohibitions

- Application-specific logic must not enter the kernel or generic services.
- The kernel must not hardcode provider, driver, gateway, model, chain, payment, or application names.
- No service call may run without trace.
- No capability call may run without policy.
- Optional modules must not become required base OS dependencies.
- Web and CLI must not become long-term owners of system orchestration.
- Providers must not be constructed outside approved composition roots.
- Logs and snapshots must not contain raw secrets, prompts, manifests, package bytes, credentials, private keys, unbounded payloads, or unsanitized provider output.
- Static registries and process locks must document lifecycle, restart
  semantics, and test-isolation strategy. Low-risk static state should move to
  explicit composition-root state when behavior can be preserved.

## Change Rules

Any boundary change must:

1. Create or update the OpenSpec proposal, design, tasks, and specs.
2. Run GitNexus impact analysis.
3. Update this document, the architecture governance document, or the serviceization admission list.
4. Preserve caller contracts; breaking changes must be expressed through clear version boundaries.
5. Pass targeted tests, boundary tests, and audit replay checks.

If a change cannot explain why it belongs in a lower layer, it must remain in a higher layer.

Provider-family extraction readiness must be checked before moving runtime-host
bridges into dedicated service crates: stable typed contracts, service
descriptor completeness, isolated adapter Strategy, explicit state machine,
sanitized audit trail, unavailable provider behavior, and compatibility tests.
