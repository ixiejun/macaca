# Change: Refactor Architecture Smell Roadmap

## Why

`tasks/smell-report-2026-06-18-1124.md` identified that Macaca's architecture is fundamentally aligned with the microkernel/service-runtime model, but several residual smells still threaten long-term maintainability: shell-owned task decomposition semantics, a runtime-host coupling hotspot, near-limit source files, oversized boundary tests, repeated linear scans, dense protocol DTO modules, process-local static state, and text/name-based routing.

These smells should be resolved through the existing Macaca governance model rather than isolated cleanup. The work must preserve the microkernel boundary, serviceization allowlist, and thin-shell contracts while making the smell prevention rules executable and auditable.

## What Changes

- Move task decomposition semantics out of `macaca-web` and behind a service-owned, provider-neutral command/strategy path.
- Split oversized serviceization/boundary tests into smaller policy-specific gates with shared fixtures.
- Add a 450-line advisory source-size gate while preserving the existing hard 500-line OS-layer gate.
- Add a static shell semantic ownership gate that rejects shell-owned task/planning semantics unless the shell only delegates through SDK/facade/service clients.
- Split near-limit runtime-host provider modules by descriptor, state, command handler, adapter, and test-fixture ownership.
- Add request-local indexes for repeated membership scans in capability catalog, route projection, task dependency selection, and skill mapping paths.
- Document and, where feasible, encapsulate `OnceLock`/static registry lifecycle boundaries.
- Move repeated integration-test fixtures into small support modules.
- Define an extraction-readiness process for mature runtime-host provider families before they can become dedicated service crates.
- Split dense `macaca-proto` DTO modules by command family and keep semantic behavior out of protocol objects.
- Replace remaining text/name-based routing with typed capability descriptors, declarative mappings, and audited fallback policies.
- Add an architecture-smell CI lane that initially reports complexity/coupling trends without failing the build, while hard boundary gates remain failing tests.

## Impact

- Affected specs:
  - `web-cli-thin-shell-completion`
  - `service-runtime`
  - `serviceization-dependency-gate`
  - `serviceization-escape-hatches`
- Affected code areas:
  - `crates/shells/macaca-web/src/loop_manager/*`
  - `crates/services/macaca-task/*` or the canonical task/autonomy service command owner
  - `crates/runtime/macaca-runtime-host/src/*`
  - `crates/foundation/macaca-proto/src/*`
  - `crates/tests/macaca-integration-tests/tests/*`
  - `crates/tests/macaca-integration-tests/tests/protocol_service_dependency_boundaries/*`
  - smell/architecture audit tooling or integration-test gates
- Affected governance:
  - `docs/macaca-os-architecture-governance.md`
  - `docs/macaca-os-microkernel-boundaries.md`
  - `docs/macaca-os-serviceization-allowlist.md`

## Non-Goals

- Do not introduce application-specific, Codex-specific, provider-specific, model-specific, driver-specific, chain-specific, gateway-specific, or business-domain routing logic.
- Do not move task planning, execution, review, recovery, or decomposition semantics into the kernel, SDK, Web, CLI, or frontend.
- Do not split runtime-host provider families into new crates until an extraction-readiness gate proves contracts, tests, service replacement mechanics, trace/audit behavior, and rollback path are stable.
- Do not make the architecture-smell CI lane fail on complexity trend findings in its first implementation; only existing hard boundary rules and new hard semantic gates fail.

## Evidence Source

- Primary smell report: `tasks/smell-report-2026-06-18-1124.md`
- Governance documents:
  - `docs/macaca-os-architecture-governance.md`
  - `docs/macaca-os-microkernel-boundaries.md`
  - `docs/macaca-os-serviceization-allowlist.md`
  - `docs/design_patterns.md`
