# Change: Remediate OS Constitution Compliance Debt

## Why

The 2026-07-08 full-system constitutional audit
(`docs/2026-07-08-macaca-os-constitution-compliance-audit.md`) verified, with
file:line evidence across all 27 workspace crates, four systemic classes of debt
that violate the three governing constitutions
(`docs/macaca-os-architecture-governance.md`,
`docs/macaca-os-microkernel-boundaries.md`,
`docs/macaca-os-serviceization-allowlist.md`):

1. **Ungated side effects (most severe).** Skill/tool process execution and
   arbitrary filesystem read/write, plus every Task write command, reach their
   side effects with no policy / entitlement / resource / budget / trace gate;
   several evidence and readiness checks are *fail-open* (`default() == Accepted`,
   `None` readiness treated as allowed).
2. **Business semantics leaking below their owning layer.** 127 domain-pack
   contract files (~43k lines, incl. approval classification & accounting report
   semantics) sit in foundation `macaca-proto`; payment contracts sit in
   foundation `macaca-persist`; Web3 types sit in foundation `macaca-ipc`; the
   `macaca-web` shell still constructs prompts and owns retry/replan/terminal-state
   repair; `macaca-cli` bypasses the SDK with hand-rolled HTTP clients.
3. **Provider/model-name hardcoding below the application layer.** LLM pricing
   table branches on model name; a `provider_name == "minimax"` branch hardcodes
   a vendor URL; proto default config hardcodes DashScope/Milvus/Telegram;
   `macaca-framework` embeds a DashScope formatter; `macaca-skill/provisioner`
   uses `concat!` to split literals *specifically to evade the gate*.
4. **Correctness defects.** 3 UTF-8 byte-slice panics (crash on Chinese/emoji
   input), scheduler retry-backoff bypass + run-id lexical mis-ordering, heartbeat
   overwriting an in-flight run, event-log falsely reporting durability, context
   truncation producing orphaned tool messages (API 400), non-idempotent retry in
   `service_router` (can double-charge payments / re-deploy contracts), duplicate
   task claim races, and a 6-lock-across-await deadlock risk.

An immediate blocker also exists: uncommitted `ai_common.rs` (526 lines) breaks
the `os_layer_file_size_gate` hard gate right now.

## What Changes

This change introduces reusable, provider-neutral OS mechanisms and hardens the
executable gates so the debt cannot silently return. All new mechanisms are
OS-layer primitives with no application-specific logic and no hardcoded
application/provider names.

- **NEW `os-side-effect-guard`** — a Decorator that enforces the canonical
  `trace → policy → entitlement → budget → resource → execute → audit` order for
  every privileged side effect, with a shared **fail-closed** readiness type.
  Skill/tool process execution and filesystem access are re-homed behind
  runtime-host providers wrapped by this guard.
- **NEW `observability-sanitization`** — a foundation module providing
  `redact_text`, UTF-8-safe `safe_truncate`, `bounded_refs`, and metadata
  allow-listing; adopted across LLM/memory/gateway/tools/kernel observability
  surfaces. Fixes the 3 byte-slice panics and the raw-payload leaks.
- **NEW `service-state-integrity`** — a generic `TransitionMatrix<S>` primitive,
  idempotency-aware retry, CAS/lease-based claim, and crash-recovery invariants
  for Task/Scheduler/Heartbeat/Autonomy.
- **NEW `provider-absence-contract`** — every provider-backed capability returns
  a structured unavailable/unsupported/denied state instead of fake success,
  silent fallback, crash, or hang.
- **NEW `service-resilience-hygiene`** — lock-poison tolerance and bounded memory
  growth (retention/TTL) as OS-wide invariants.
- **MODIFIED (ADDED requirements) `serviceization-dependency-gate`** — replace
  the provider-construction denylist with pattern matching + mandatory
  registration; add an anti-literal-splitting rule; add a `use`-level boundary
  scan; enforce file-size gate on contract files.
- **MODIFIED (ADDED requirements) `microkernel-boundary-purity`** — foundation
  `macaca-proto` / `macaca-ipc` / `macaca-persist` purity: extract domain-pack
  contracts to a dedicated crate, feature-gate Web3/NATS, move the payment store
  out, neutralize proto default config.
- **MODIFIED (ADDED requirements) `web-cli-thin-shell-completion`** — shell owns
  no prompt construction, retry, replan, or terminal-state repair; CLI holds no
  direct HTTP client.
- **MODIFIED (ADDED requirements) `sdk-system-facade`** — a generic domain-pack
  preflight builder skeleton; table-driven client tests.

**BREAKING**: domain-pack contract types move from `macaca-proto` to a new
`macaca-domain-pack-contracts` crate (re-exported for a deprecation window);
LLM `from_config` provider construction moves to the host composition root.

GitNexus impact analysis will be run per the mandatory workflow, but per the
requester's instruction its CRITICAL/HIGH warnings for this remediation are
**recorded for the record only** and do not block execution (see `design.md`
§Open Questions and `tasks.md` §0).

## Impact

- **Affected specs**: `os-side-effect-guard` (new), `observability-sanitization`
  (new), `service-state-integrity` (new), `provider-absence-contract` (new),
  `service-resilience-hygiene` (new), `serviceization-dependency-gate`,
  `microkernel-boundary-purity`, `web-cli-thin-shell-completion`,
  `sdk-system-facade`.
- **Affected code (foundation)**: `macaca-proto` (config/root.rs,
  domain_pack_contract/*), `macaca-ipc` (web3_bridge, Cargo.toml),
  `macaca-persist` (event_log.rs, payment_store.rs); new
  `macaca-domain-pack-contracts`.
- **Affected code (kernel)**: `macaca-kernel` (logging.rs, alert.rs,
  execution_port.rs).
- **Affected code (services)**: `macaca-task`, `macaca-scheduler`,
  `macaca-heartbeat`, `macaca-scheduled-agent-task`, `macaca-autonomy-evolution`,
  `macaca-llm`, `macaca-memory`, `macaca-context`, `macaca-driver`,
  `macaca-skill`, `macaca-gateway`, `macaca-tools`.
- **Affected code (runtime/shell)**: `macaca-runtime-host` (service_router.rs,
  execution_control_runtime.rs, skill_service_provider_merge.rs, queue.rs),
  `macaca-runtime` (context_window.rs, loop_detector.rs), `macaca-framework`
  (formatter.rs), `macaca-app` (llm_proxy.rs), `macaca-web` (loop_manager/*),
  `macaca-cli` (*_operations).
- **Affected gates/tests**: `crates/tests/macaca-integration-tests/tests/`
  (protocol_service_dependency_boundaries, os_layer_file_size_gate,
  sdk_no_provider_construction_gate, new side-effect/sanitization/state gates).
