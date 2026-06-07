# Change: Add Autonomy Evolution Control Plane

## Why

Macaca now proves a governed Skill-level self-evolution loop, but the platform
does not yet own a generic autonomous policy loop that can discover, generate,
evaluate, promote, monitor, and roll back improvements across replaceable target
types. Treating Skill materialization alone as "complete self-evolution" would
overstate the current system and would leave evaluation, release safety, and
OS-code governance outside the audited service boundary.

## What Changes

- Add a service-owned Autonomy Evolution Control Plane capability with typed
  run lifecycle commands/results.
- Model evolution run lifecycle as an explicit State machine from observation
  through proposal, admission, quarantine, benchmark preparation, canary,
  promotion, monitoring, rollback, rejection, or inconclusive close.
- Introduce a provider-neutral Target Adapter Strategy contract so Skills are
  the first target type while application capability packs, task/context policy,
  and OS-code proposal adapters can be added later.
- Require trace, policy, sanitized audit, scope, and bounded evidence refs
  before any side-effecting transition.
- Add SDK/SystemFacade unavailable behavior so shells and applications receive
  explicit unavailable results when the control plane provider is absent.
- Keep Web, CLI, and frontend as thin diagnostic or trigger adapters only.

## Impact

- Affected specs: `autonomy-evolution-control-plane`
- Affected code:
  - Autonomy/control-plane service contracts and DTOs.
  - Runtime-host provider skeleton and target adapter registry.
  - SDK/SystemFacade focused client and unavailable implementation.
  - Future thin Web/CLI diagnostics after the service boundary exists.
  - Tests for lifecycle transitions, policy-required transitions, unavailable
    behavior, dependency boundaries, and OpenSpec validation.

## Non-Goals

- Do not move self-evolution orchestration into the kernel.
- Do not make Web, CLI, or frontend classify, score, promote, roll back, or
  benchmark candidates.
- Do not implement normalized benchmarking, canary release, production
  Store/EventLog migration, or OS-code mutation in this first change.
- Do not branch on application names, workflow names, provider names, model
  names, driver names, or business domains.
- Do not persist raw prompts, raw provider payloads, raw manifests, package
  bytes, secrets, credentials, private keys, raw signatures, or unbounded output.
