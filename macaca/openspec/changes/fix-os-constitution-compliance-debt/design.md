## Context

The 2026-07-08 audit produced ~90 evidence-backed findings spanning every layer.
They are not independent bugs; they cluster into a small number of *missing OS
primitives*. Rather than patch each call site, this design introduces the
primitives the constitutions already imply, then re-homes offending code behind
them. This keeps the fix provider-neutral, testable by executable gates, and
resistant to regression.

Constraints (from the three constitutions and the requester):
- No application-specific logic and no hardcoded application/provider/model names
  below the application layer.
- Every cross-boundary operation is a typed Command/Result; every service call
  carries trace; policy runs before side effects.
- Every new mechanism must be replaceable (Strategy/Adapter) and observable
  (trace/audit/log) with bounded, sanitized output.
- All new Rust code carries detailed English comments explaining function and
  operating principle, and logs key execution nodes.
- GitNexus CRITICAL/HIGH warnings for this remediation are recorded, not gating.

## Goals / Non-Goals

Goals:
- Make privileged side effects impossible without a fail-closed gate chain.
- Provide one shared, correct sanitization/truncation surface (no byte-slice
  panics, no raw-payload leaks).
- Provide one generic state-transition/idempotency/retry primitive.
- Restore foundation-layer purity (contracts, payment, web3, config).
- Harden the executable gates so the debt cannot silently reappear.

Non-Goals:
- Implementing new business domains or packs (packs stay in `crates/packages/`).
- Changing user-visible task/goal/chat behavior beyond correctness fixes.
- Real cryptographic payment/skill execution (out of scope; only contracts move).
- Resolving GitNexus warnings for pre-existing symbols (deferred, recorded).

## Decisions

### D1 — `SideEffectGuard` Decorator (os-side-effect-guard)
A single generic decorator wraps any `impl SideEffectHandler`. It enforces the
order `require_trace → policy_decision → entitlement/budget → resource_reserve →
handler.execute → audit_write`. Readiness is expressed by a shared
`Readiness { Ready, NotReady(reason), Unknown }` type whose gate rule is
**fail-closed**: only `Ready` proceeds; `Unknown`/`NotReady` return structured
`Denied`. This directly replaces the fail-open `Option<bool>` patterns in
`macaca-skill` (`proposal_lifecycle.rs:175`, `proposal_processing.rs:183`,
`evolution.rs:48`) and the ungated executors in `macaca-tools/builtin.rs` and
`macaca-skill/tool.rs`.
- Pattern: Decorator + Strategy (policy/entitlement are injected ports).
- Alternatives considered: per-service inline checks (rejected — that is the
  current state that produced the debt) ; a macro (rejected — hides control flow
  from trace/audit and is harder to test).

### D2 — Foundation `sanitize` module (observability-sanitization)
A dependency-free module in `macaca-proto` (or a tiny `macaca-sanitize`
foundation crate) exporting: `safe_truncate(&str, max) -> &str` using
`floor_char_boundary`/`char_indices` (never a byte index); `redact_text` (the
existing `macaca-memory` implementation, promoted); `bounded_refs`; and
`metadata_allowlist`. Redaction uses a **structural allow-list** (accept only
controlled id/URI shapes; reject/hash everything else) rather than the current
literal-word **deny-list**, which the audit showed lets real `sk-…` values
through (`autonomy governance_ledger.rs:416`).
- Pattern: pure functions + a single source of truth; adopted via a new gate
  forbidding raw `.text().await` concatenation into error/log strings.

### D3 — `TransitionMatrix<S>` + idempotent retry (service-state-integrity)
A declarative legal-transition table (`Specification` pattern) shared by Task,
Scheduler, Heartbeat, Autonomy. `transition(current, target)` returns structured
`Conflict` for illegal moves, fixing terminal-state re-transition, ghost
retries, and the heartbeat coalesce-over-Running defect. Retry becomes
idempotency-aware: `service_router` reads an `idempotent` flag from the service
descriptor and only retries idempotent operations (fixes double-charge/double-
deploy). Claim uses CAS/lease (version-checked write) to remove the read-modify-
save race. Run identifiers are zero-padded / numeric-keyed to restore ordering.

### D4 — Foundation purity extraction (microkernel-boundary-purity)
- Domain-pack contracts move `macaca-proto/domain_pack_contract/*` →
  `crates/foundation/macaca-domain-pack-contracts` (re-exported from proto for a
  deprecation window). Approval-classification/bounds/reports semantics move up
  to packs or Task/Autonomy services; proto keeps only provider-neutral DTOs,
  command names, error types, and a self-registration registry (data-driven,
  replacing the `("finance","crypto") => …` match).
- `payment_store.rs` moves to the payment service crate; foundation keeps neutral
  KV/Memento primitives.
- Web3 bridge + `async-nats` become `#[cfg(feature)]`-gated optional deps.
- `RootConfig::default()` returns neutral/empty values; concrete provider values
  live in `config/default.toml`.

### D5 — Shell semantic re-homing (web-cli-thin-shell-completion)
Prompt construction, retry, replan, and terminal-state repair in `macaca-web`
become four typed Task/Autonomy service commands
(`build_task_execution_prompt` or a structured execution command, `retry_task`,
`build_followup_planning_prompt`, `cancel_partial_goal_tasks`). Goal evaluation
failure returns an explicit outcome instead of a shell-side "mark complete by
default". CLI's three reqwest clients are replaced by SDK clients; `reqwest` is
removed from CLI.

### D6 — Gate hardening (serviceization-dependency-gate)
Provider-construction gate switches from an 11-token deny-list to a
naming-pattern matcher (`*ServiceProvider::`, `*Provider::new`) plus a mandatory
registration meta-test, and adds an explicit rule forbidding literal-splitting
evasion (`concat!("claude","-code")`). A `use`-statement-level scan complements
the Cargo-metadata dependency gate.

## Risks / Trade-offs

- **Contract crate move is BREAKING for imports.** → Re-export from `macaca-proto`
  during a deprecation window; migrate call sites in the same change; dependency
  gate asserts no new `macaca-proto -> domain contract` reverse edge afterwards.
- **SideEffectGuard on the hot path adds latency.** → Guard is O(1) plus injected
  port calls that already exist logically; policy/entitlement ports default to
  local in-memory deciders; measured against `/api/chat/v2` regression.
- **Fail-closed may deny previously-allowed flows.** → Provide explicit
  bootstrap/self grants; cover with regression scenarios so absence of a grant is
  a visible test failure, not a silent breakage.
- **Wide mechanical edits (191 lock `.expect`, byte slices).** → Isolate each
  mechanical sweep in its own task/PR with a gate that fails on reintroduction.

## Migration Plan

Staged, matching `tasks.md`: Stage 0 unblocks CI (split `ai_common.rs`); Stage 1
lands P0 safety/crash fixes; Stage 2 sanitization + gate hardening + structured
unavailable; Stage 3 side-effect guard + trace closure; Stage 4 state-machine
correctness; Stage 5 boundary extraction (contracts/shell/foundation); Stage 6
resilience hygiene + lifecycle completeness. Each stage: `cargo check
--workspace`, full gate suite, targeted unit/integration tests, and (for OS-layer
behavior) an OpenSpec-tracked delta. Rollback is per-stage because the primitives
are additive (old paths deleted only after the new path passes its gate).

## Open Questions

- OQ1: Are LLM/driver/context provider ports always reached through the
  runtime-host decorator, or can `impl LlmProvider for LlmRouter` be direct-called
  to bypass it? (Audit S21 — needs confirmation; may require a port-signature
  trace argument.)
- OQ2: Is the Milvus backend (`macaca-memory/vector.rs`) only ever an
  unauthenticated local instance, or must auth/TLS be added?
- OQ3: Keep `macaca-app`'s `LlmProxy` as a recorded mediator boundary, or route
  it through SDK/service edges?
- OQ4 (recorded, non-gating): GitNexus reports CRITICAL/HIGH blast-radius for
  several touched symbols. Per requester instruction these are logged in each
  task's notes and NOT treated as blockers for this change.
