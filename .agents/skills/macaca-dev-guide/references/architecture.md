# Macaca Architecture Reference

This file is intentionally a compact reference. The authoritative architecture
documents are:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`
- `docs/openharmony-microkernel-architecture-for-macaca-agent-os.md`

If this file disagrees with those documents or current code, trust the stable
governance documents and current code. Historical refactor plans are useful for
understanding evolution, but not as current implementation checklists.

## Current Layered Workspace

```text
macaca/crates/foundation/
  macaca-proto          # Provider-neutral DTOs, ABI, service/package/capability types.
  macaca-ipc            # Local-first service bus and trace-required middleware.
  macaca-persist        # Persistence, event log, snapshots.

macaca/crates/kernel/
  macaca-kernel         # Microkernel primitives: identity, registry, policy facade,
                        # scheduler/session/task/resource/audit primitives.

macaca/crates/services/
  macaca-llm            # LLM service/provider family.
  macaca-memory         # Memory service/provider family.
  macaca-context        # Context service/provider family.
  macaca-task           # Task/planner/review service contracts and behavior.
  macaca-driver         # Driver service/provider family.
  macaca-skill          # Skill service/provider family.
  macaca-gateway        # Gateway service/provider family.
  macaca-tools          # Tool abstractions that should route through service-backed calls.

macaca/crates/runtime/
  macaca-runtime        # Agent runtime loop primitives.
  macaca-framework      # Application/runtime framework adapters.
  macaca-runtime-host   # ServiceRuntime, built-in providers, module bootstrapping,
                        # WASM host imports, service decorators, diagnostics.

macaca/crates/application/
  macaca-agent          # Agent abstractions/state.
  macaca-app            # Application manifests, package metadata, ABI, GenUI,
                        # compatibility/admission checks.

macaca/crates/facade/
  macaca-sdk            # SystemFacade and focused developer/shell-facing clients.

macaca/crates/shells/
  macaca-web            # Axum API, SSE, shell adapters, temporary composition root.
  macaca-cli            # CLI adapter.

macaca/crates/tests/
  macaca-integration-tests
```

## Ownership Summary

| Layer | Owns | Must Not Own |
| --- | --- | --- |
| Foundation | Shared contracts, IPC, persistence | Provider behavior, app semantics |
| Kernel | System invariants and primitive facades | Concrete providers, prompts, workflows |
| Services | Replaceable capability families | Presentation shell state, app-specific logic |
| Runtime Host | Provider wrappers, lifecycle, decorators, modules | Business workflows, raw UI state |
| Application | Manifest, ABI, package, app lifecycle, GenUI metadata | Generic OS policy bypass |
| SDK/Facade | Provider-neutral clients and Null Object behavior | Provider construction |
| Shells | Input/output adapters, rendering, approval, diagnostics | OS semantics |

## Stable Product Vision

Macaca is also a highly intelligent 24/7 autonomous application platform. Its
architecture should let agents accept goals, plan work, call services, delegate,
observe progress, recover after interruptions, and finish tasks with minimal
human micromanagement.

Human involvement should be explicit and high-value: policy, safety, approvals,
budget, entitlement, product judgment, and corrections. Routine planning,
execution, verification, retry, and trace reporting belong to the platform.

## Stable System-Service Families

- LLM, Memory, Context.
- Task, planner, execution control, review, recovery, retry.
- Driver, Skill, MCP catalog and invocation.
- Application registration, manifest, lifecycle, runtime state.
- Gateway ingress and external event bridges.
- Store, package index, entitlement, license, metering.
- Payment, A2A quote/intent/approval/receipt/settlement.
- Web3, wallets, chain clients, EVM.
- Trace, audit, diagnostics, GenUI/UI runtime surfaces.

## Stable Acceptance Gates

For OS-layer work, prove:

- YAML, WASM, and GenUI application paths still run or return structured unavailable states.
- `/api/chat/v2` session creation/recovery does not regress.
- Task boards remain scoped by session.
- Trace and audit evidence is replayable after refresh.
- Optional services and modules may be absent.
- Dependency boundary tests still pass.
- Logs and snapshots are bounded and sanitized.
