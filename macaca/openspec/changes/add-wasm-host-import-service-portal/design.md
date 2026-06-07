## Context
The default in-process WASM provider can execute minimal exports and enforce sandbox/resource policy. Host imports now need a separate Bridge so guest import ABI can reach Macaca services through `ServiceRuntime` without exposing concrete service implementations.

## Goals / Non-Goals
- Goals: provide host import command DTOs, validate trace/payload/capability metadata, route service calls through `ServiceRuntime`, sanitize results, and log auditable allow/deny/failure events.
- Non-Goals: implement real business providers for every system service, execute payment/web3 operations, add raw guest IO, or let guest code bypass ServiceRuntime.

## Design Pattern Selection
- Command: each import becomes a `WasmHostImportCommand` with category, target service, operation, payload, trace, and bounded metadata.
- Bridge: `WasmHostImportBridge` translates application ABI imports to `ServiceRuntime::call` while keeping runtime provider code independent of service implementations.
- Proxy: SDK/guest-facing helpers construct service-call commands while the host owns actual dispatch.
- Chain of Responsibility: bridge validation runs trace, payload size, capability, policy/runtime, and service availability checks in order.
- Observer: every request, denial, unavailable service, and completed call emits sanitized audit logs and result metadata.

## Technical Decisions
The bridge lives in `macaca-runtime-host` because only the host owns `ServiceRuntime`. Provider-neutral DTOs live in `macaca-proto` so SDK tests, admission, and future providers share one vocabulary. The default provider receives an optional bridge handle; when no bridge is installed, imports fail closed with structured unavailable results.

## Redaction
Bridge reports and logs may include trace id, application id, ability id, import name, service id, operation, status, reason code, and bounded payload byte counts. They must not include raw guest payloads, raw prompts, raw WASM bytes, environment values, filesystem paths, network targets, provider secrets, private keys, or backend responses before bounding.
