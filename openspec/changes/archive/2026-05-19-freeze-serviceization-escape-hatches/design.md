## Context
The 2026-05-16 serviceization audit found compatibility paths that still bypass
stable Macaca OS boundaries. Removing them requires separate ownership tracks,
but the platform first needs a Specification-pattern gate that stops new escape
hatches from entering production code.

## Goals / Non-Goals
- Goal: prevent new production references to known direct runtime/provider paths.
- Goal: make each dependency allowlist row traceable to an owner, caller,
  replacement, expiry, and validation command.
- Goal: preserve existing runtime behavior while migration tracks proceed.
- Non-goal: remove kernel provider compatibility, Web direct runtime reads, CLI
  Web coupling, or domain-pack implementations in this change.

## Decisions
- Use an integration test as the executable specification because it runs in the
  same verification path as Route C dependency gates and does not require a new
  dependency.
- Keep allowlist metadata in Rust, not markdown parsing, so CI diagnostics are
  deterministic and audit-friendly.
- Treat approved migration surfaces as explicit and narrow. Future workers must
  update OpenSpec before adding a new migration surface.
- Use a Port/Adapter split for kernel agent execution. The kernel owns only a
  provider-neutral `AgentExecutionPort`; the temporary legacy
  `Agent::run(llm, tools, services)` bridge lives in the application-agent layer
  where the legacy agent ABI is already defined.
- Use a Null Object execution port for service-client construction that has no
  agent execution bridge yet. It returns a structured unavailable error and logs
  the missing bridge instead of faking success.
- Use a Repository/Port split for kernel persistence. Audit logging, execution
  queue recovery, fork recovery, and deprecated kernel A2A payment compatibility
  depend on kernel-owned persistence ports; concrete Redb and in-memory stores
  remain in composition roots, service/foundation crates, or tests.
- Use focused SDK clients for Web toolkit assembly. Driver tool discovery now
  treats client failure as a structured unavailable state instead of falling
  back to direct runtime internals, and MCP definition reads move through the
  MCP service snapshot command with optional serialized definition payloads.

## Design Patterns
- Specification: forbidden references and allowlist metadata are executable
  architectural requirements.
- Facade: diagnostics point callers toward `SystemFacade` or focused clients.
- Command: lifecycle and catalog operations are required to move toward typed
  service commands.
- Adapter/Bridge: remaining direct runtime access is named as migration-only
  bridge debt, not stable ownership.
- Facade/Command: Web toolkit assembly consumes Driver and MCP focused clients
  so shell code stays an adapter over service DTOs instead of a runtime owner.
- Port/Adapter: kernel execution depends on an abstract execution port while
  provider-shaped legacy execution is adapted outside the kernel core.
- Null Object: service-client-only kernel construction fails explicitly when
  legacy execution is invoked without a service execution bridge.
- Repository/Memento: kernel persistence ports store replayable audit, queue,
  fork, and payment transition mementos without binding the kernel to one
  database implementation.
- Decorator: later service calls retain trace, audit, and policy behavior at the
  boundary rather than in shells.

## Risks
- Existing migration files may need temporary approval entries. This is
  acceptable only when they are explicitly listed as migration surfaces.
- String scanning can produce false positives. The first gate therefore scans
  conservative tokens and excludes tests/fixtures so it freezes production debt
  without breaking legitimate examples.
