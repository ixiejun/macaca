# Change: Refactor macaca-sdk with design pattern primitives

## Why

`macaca-sdk` is the developer-facing boundary for declaring agents and registering them into Agent OS. Today the SDK builder directly constructs runtime agents, persona reuse is copy-based, validation is a monolithic method, and registry helpers directly depend on `Kernel`.

Macaca is an Agent OS foundation. SDK primitives need stable declaration, validation, persona, registration, and trace-policy boundaries so later application/framework migrations can reuse them without app-specific code or hardcoded workflow assumptions.

## What Changes

- Add `AgentSpec` as the SDK builder product while preserving existing `DeclarativeAgent` behavior.
- Add persona prototype and override primitives.
- Add SDK validation chain primitives and route current validation through the default chain.
- Add `MacacaSdk` facade and registry adapter primitives.
- Require SDK-built agent specs to carry trace policy metadata.
- Keep old helper functions callable but mark replaced registry helpers deprecated after the facade exists.

## Impact

- Affected specs: `macaca-sdk-patterns`
- Affected code: `macaca-sdk`
- Compatibility impact: `AgentBuilder::build`, `AgentBuilder::build_with_manifest`, `register_from_config`, and `register_from_file` remain callable and preserve current behavior.
- Non-impact: no app runtime behavior change; no kernel registration behavior change; no trace/EventLog/SSE/task/driver/skill/MCP behavior change.
