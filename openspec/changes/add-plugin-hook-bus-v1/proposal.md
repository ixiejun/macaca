# Change: Add Plugin Hook Bus v1

## Why

Macaca plugins must be able to participate in agent, application, task, tool, prompt/context, memory, LLM, gateway, approval, and session lifecycle events without bypassing service boundaries or mutating core state unsafely.

Without a typed Hook Bus, each service will grow ad hoc extension points, making plugin behavior untraceable, unbounded, and inconsistent.

## What Changes

- Add typed plugin hook contracts for observer, mutating, blocking, and approval hooks.
- Add a runtime-host-owned Hook Bus with priority ordering, timeout policies, failure policies, schema-validated results, and trace/audit events.
- Define the first core hook set: agent lifecycle, application lifecycle, task lifecycle, tool call, prompt/context build, memory ingest, LLM call, gateway message, approval lifecycle, and session lifecycle.
- Ensure hooks cannot directly access internal runtime objects or bypass permission/resource admission.
- Add unavailable-safe behavior for disabled or missing hook plugins.

## Impact

- Affected specs: `plugin-hook-bus`
- Affected code: `macaca-proto`, `macaca-runtime-host`, `macaca-framework`, `macaca-task`, `macaca-context`, `macaca-memory`, `macaca-gateway`, `macaca-web`, integration tests
- Affected tests: hook contract tests, hook runner tests, framework/service integration tests, trace/audit tests

## Required Governance

- Hooks must be typed and explicit.
- Hook execution must be bounded by timeout and failure policy.
- Mutating hooks must return schema-validated results.
- Blocking hooks must return structured allow/block/require-approval decisions.
- Hooks must emit trace/audit records.
- No hook may receive secrets, raw credentials, private keys, or unbounded sensitive payloads.
