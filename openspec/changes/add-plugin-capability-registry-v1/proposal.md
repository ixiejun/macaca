# Change: Add Plugin Capability Registry v1

## Why

Plugin Runtime v0 can store plugin-provided service descriptors, but Macaca still lacks a complete capability plane where plugins can declare, discover, register, conflict-check, and expose capabilities without launching runtime code.

Without a capability registry, Driver, Skill, MCP, Gateway, Memory, Context, LLM Provider, Observability, Tool, Hook, HTTP Route, and CLI Command extensions will grow separate registries and bypass Route C service boundaries.

## What Changes

- Add provider-neutral plugin capability descriptors for tool, hook, driver, gateway, skill, MCP, memory, context, LLM provider, observability, HTTP route, CLI command, and custom capabilities.
- Add contract-first discovery so Macaca can resolve capability ownership from manifests/repository snapshots without starting plugin runtime.
- Add conflict policies for tool names, exclusive provider slots, gateway routes, HTTP routes, CLI commands, and custom slots.
- Add canonical built-in adapter registration for existing built-in services and mark replaced direct descriptor construction paths as deprecated.
- Add capability registration, activation, query, call-routing skeleton, and cleanup semantics.
- Ensure all capability calls require trace context and permission/resource admission.

## Impact

- Affected specs: `plugin-capability-registry`
- Affected code: `macaca-proto`, `macaca-kernel`, `macaca-runtime-host`, `macaca-driver`, `macaca-skill`, `macaca-gateway`, `macaca-memory`, `macaca-context`, `macaca-llm`, `macaca-sdk`, integration tests
- Affected governance: Route C dependency gate and serviceization allowlist only if new dependency edges are introduced
- Affected tests: capability descriptor tests, registry ownership cleanup tests, built-in adapter tests, Route C dependency boundary tests

## Required Governance

- Capability descriptors are data contracts, not business logic.
- Kernel stores ownership/invariants only.
- Runtime-host coordinates registration and conflict policy.
- Concrete services own capability behavior.
- No hardcoded provider/app/driver/gateway/model/chain/business names.
- Capability registration and call skeletons must be traceable, auditable, and unavailable-safe.
