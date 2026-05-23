# Change: Complete Self-Evolving Skill OS

## Why

Macaca has landed the first service-owned pieces of self-evolving skills:
governance telemetry, deterministic dry-run, draft proposal snapshots, alias
resolution, lifecycle metadata commands, and a read-only operations surface.
The research in `docs/macaca-agent-self-evolving-skills-research.md` still
requires a durable, auditable, policy-gated implementation path before Macaca
can safely let autonomous agents grow, curate, merge, and retire skills over
long-running 24/7 operation.

The remaining work must be planned as one coherent OS capability instead of a
collection of ad hoc commands. Skill self-evolution is a system service concern:
the kernel must only route typed calls and preserve trace/audit invariants,
runtime-host providers must remain replaceable, SDK/Web/CLI must stay facades
and adapters, and no application-specific workflow or provider-specific rule may
enter generic OS code.

## What Changes

- Define the remaining self-evolving Skill OS contract in one proposal covering
  durable governance storage, lifecycle completion, rich provenance,
  telemetry, task-completion integration, draft promotion/rejection, safe skill
  file mutation, curation runs, semantic review providers, umbrella merge,
  context composer filtering, package ownership, approval UI mutations,
  boundary gates, and scheduler/task alias behavior.
- Require Store/EventLog-backed governance as the durable source of truth for
  lifecycle, provenance, telemetry, aliases, proposals, curation runs, reports,
  rollback mementos, and sanitized audit evidence.
- Extend the Skill service contract with trace-required, policy-gated command
  surfaces for evolution, curation, lifecycle, alias, mutation, approval, and
  rollback while preserving unavailable/unsupported/denied states.
- Add a staged implementation plan that lands durable read models before any
  active file mutation, then introduces draft-only apply paths, policy-gated
  promotion, deterministic curation, optional semantic review, and shell
  approval adapters.
- Keep every capability generic to all applications and agents. The design
  prohibits application, workflow, model, driver, provider, gateway, chain, or
  business-domain hardcoding.

## Impact

- Affected specs: `skill-governance-curation`
- Affected service families: Skill, Store/EventLog, Task, Context, Memory,
  Knowledge, Policy, Entitlement, Scheduler/Autonomy, Service Runtime.
- Affected layers:
  - Kernel: typed service call, policy facade, trace/audit identity only.
  - Services: Skill owns evolution and curation semantics; Store/EventLog owns
    durable records; Context/Task/Memory/Knowledge integrate only through
    service calls.
  - Runtime host: built-in provider adapters, composition roots, provider
    factories, sanitized diagnostics.
  - SDK/SystemFacade: provider-neutral clients, typed DTOs, Null Object
    behavior.
  - Web/CLI/frontend: approval, report, diagnostics, refresh, and rollback
    adapters only.
- Affected code areas in later implementation slices:
  `macaca-skill`, `macaca-runtime-host`, `macaca-sdk`, `macaca-context`,
  task/autonomy services, store/event-log providers, `macaca-web`, frontend
  operations surfaces, and boundary/integration tests.

## Architecture Constraints

- Must comply with `macaca/docs/macaca-os-architecture-governance.md`,
  `macaca/docs/macaca-os-microkernel-boundaries.md`, and
  `macaca/docs/macaca-os-serviceization-allowlist.md`.
- Must use Command, Facade, Strategy, Decorator, State, Observer, Memento,
  Specification, and Abstract Factory patterns where they clarify ownership.
- Must require trace context and sanitized audit evidence for every command.
- Must run policy, resource, package guard, and entitlement checks before any
  privileged side effect.
- Must keep optional LLM, similarity, marketplace, and package providers absent
  by default with structured unavailable behavior.
- Must never store or expose raw prompts, raw provider payloads, secrets,
  manifests, WASM bytes, package bytes, private keys, credentials, raw
  signatures, full skill bodies in governance snapshots, or unbounded task
  output.
