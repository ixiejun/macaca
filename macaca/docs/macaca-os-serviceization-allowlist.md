# Macaca OS Serviceization Admission List

## Purpose

This document is the admission constitution for serviceization and modularization in Macaca OS. It provides no architecture exemptions and no openings for boundary violations; it defines which capabilities must enter as services, plugins, application-framework capabilities, or optional modules, and which dependency directions must be rejected.

When implementation convenience conflicts with this document, this document wins.

## General Principle

Non-kernel capabilities must not enter the microkernel. Any capability that is replaceable, extensible, outsourceable, or variable by tenant or application must live behind a service boundary.

Serviceization is not moving files. It is transferring ownership. A capability is serviceized only when it has a contract, policy, trace, audit, health checks, structured errors, and replacement mechanics.

## Capabilities That Must Be Serviceized

The following capabilities must exist as system services:

- LLM calls, model routing, budget, rate limiting, degradation.
- Memory, context, vector retrieval, knowledge indexing.
- Task planning, task execution, review, recovery, retry.
- Drivers, skills, MCP tool catalogs, tool invocation.
- Application registration, application manifests, application lifecycle, application runtime state.
- Store, package index, entitlement, license, metering.
- Payment, quotes, receipts, settlement, A2A transactions.
- Gateway ingress, external message entrypoints, external event bridges.
- Web3, EVM, wallets, chain clients.
- Any third-party provider adapter.

The kernel may hold abstract identity, capability names, service names, policy decisions, and call evidence for these capabilities, but must not hold concrete implementations.

## Service Admission Conditions

Every system service must satisfy:

- Stable service identity, descriptor, command surface, result types, and error types.
- Lifecycle: register, start, health check, pause, resume, shutdown.
- Every call carries session, task, application, tenant, and trace context.
- Policy, resource, budget, and entitlement checks run before side effects.
- Sanitized trace events, audit events, and bounded snapshots are emitted.
- Structured unavailable, unsupported, denied, and failure states are returned.
- Built-in, plugin, remote, mock, and unavailable provider replacement is supported.
- Web, CLI, frontend, and application-specific code are not imported.

Code that does not satisfy these conditions cannot be merged as a system service.

## Optional Module Admission Conditions

Optional modules must satisfy:

- The base OS can still start, execute tasks, recover sessions, and query audit when the module is absent.
- The module enters through capability declarations, service registration, and policy gates.
- The module does not inject reverse dependencies into the kernel, SDK, Web, or CLI.
- All failures explicitly return unavailable, disabled, or denied.
- Logs, snapshots, and traces do not leak raw secrets, raw payloads, or unsanitized provider responses.

The presence of an optional module must not change base OS system semantics.

## Plugin Admission Conditions

Plugins must declare:

- Plugin identity, version, signature, and source.
- Provided capabilities, services, commands, and events.
- Required permissions, resources, budget, and network scope.
- Install, upgrade, enable, disable, and uninstall lifecycle.
- Trace schema, audit schema, and diagnostics schema.

Plugins must not bypass the service runtime, policy gates, resource gates, or audit chain.

## Application Framework Admission Conditions

The application framework owns application contracts, not concrete business domains. YAML, WASM, GenUI, headless applications, and app-owned UI must all enter through the unified application boundary.

The application framework may own:

- Application manifests and package metadata.
- Application ABI and runtime adapters.
- App-scoped capability requests.
- App-scoped permission declarations.
- Application lifecycle and session envelope.
- Metadata and intent validation for app-owned UI surfaces.

The application framework must not own concrete finance, crypto, office, code-generation, payment, chain, or gateway business rules.

## Rejection List

The following must be rejected:

- The kernel depends on any concrete provider implementation.
- The kernel depends on Web, CLI, frontend, gateway, or application-framework implementations.
- The SDK constructs concrete providers, runtimes, database backends, wallets, or chain clients.
- Web, CLI, or frontend defines system semantics for tasks, payments, packages, chains, drivers, skills, MCP, or application execution.
- A service provider imports a presentation shell.
- Application-specific behavior enters the kernel, SDK, or generic services.
- OS-layer routing branches on provider name, model name, driver name, gateway name, chain name, payment name, or application name.
- A service call runs without trace.
- A capability call runs without policy.
- An absent optional module crashes, hangs, silently falls back, or fakes success.
- Logs, traces, snapshots, or diagnostics contain raw secrets, prompts, manifests, WASM bytes, package bytes, private keys, credentials, raw signatures, raw provider payloads, or unbounded output.

## Executable Gates

The repository must have and continuously strengthen dependency-boundary tests. At minimum, gates must cover:

- The kernel must not depend on concrete provider implementations.
- The SDK must not depend on host composition roots or presentation shells.
- Presentation shells must not become system semantic owners.
- Service providers must not depend on presentation shells.
- Optional modules must not become required base OS dependencies.
- Workspace crates must belong to clear layers.

A gate failure is an architecture violation, not test noise.
