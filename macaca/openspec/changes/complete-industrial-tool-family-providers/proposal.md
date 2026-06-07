# Change: Complete industrial tool family providers

## Why

The Tools system is only industrial-grade if Macaca applications can perform real multi-step work through rich, generic, service-owned tool families. Contracts, planning, invocation, environments, gateway, and diagnostics are necessary but insufficient without provider-backed capabilities for real work.

## What Changes

- Add or adapt provider-backed families for file, shell, browser, web, memory, knowledge, task, scheduler, skill, MCP, media, document, communication, enterprise API, code execution, computer use, and payment/entitlement.
- Prefer existing services, MCP, plugins, gateway providers, and runtime adapters before adding new built-ins.
- Add structured unavailable providers for optional families that are not installed.
- Add end-to-end application-neutral validation that uses multiple tool families in one realistic workflow.
- Add boundary tests that prove rich tools still flow through service-owned planning, invocation, result, telemetry, and audit.

## Impact

- Affected specs: `industrial-tool-families`, `tool-capability-planning`, `tool-service-invocation`, `tool-runtime-environments`, `tool-observability`
- Affected code: provider service adapters, MCP/plugin/gateway adapters, integration tests, docs
- Depends on: all previous industrial Tools proposals

## Constraints

- No application-specific OS code.
- No provider-specific OS routing branches.
- Missing optional providers must return unavailable, disabled, unsupported, or denied states.
- A successful catalog plan is not sufficient; this proposal must prove real invocation and artifact/audit behavior.
