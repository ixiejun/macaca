# Change: Complete agent self-evolution end to end

## Why

The previous autonomy evolution changes added the required gates and service contracts, but the live path still allowed callers to provide pre-shaped evidence without one runtime-owned bridge that executes the full chain. Complete agent self-evolution needs one unattended execution path that turns observer evidence into a governed target action and immediately proves the result through replayable audit.

## What Changes

- Add a runtime-host live execution bridge that composes the autonomy evolution service, Skill materialization operator, OS-code proposal adapter, and live audit replay in one traced command.
- Add a service command for the OS-code proposal adapter so source-code evolution proposals are evaluated through the autonomy evolution service boundary instead of direct host logic.
- Keep source mutation governed: OS-code proposals may be accepted for review, quarantined, or denied, while direct mutation remains blocked until a future source-mutating provider is explicitly installed.
- Record target execution outcomes and live audit evidence so shells and tests can prove `observer evidence -> live tick -> target adapter -> governance audit` from one result.
- Preserve application neutrality: no application names, workflow names, model names, provider names, or business-domain logic are introduced.

## Impact

- Affected specs: `autonomy-evolution-control-plane`
- Affected code:
  - `macaca/crates/services/macaca-autonomy-evolution`
  - `macaca/crates/runtime/macaca-runtime-host`
  - `macaca/crates/facade/macaca-sdk`
- Governance: follows Command, Adapter/Bridge, Strategy, Observer, Memento, and Specification patterns from the Macaca OS governance documents.
