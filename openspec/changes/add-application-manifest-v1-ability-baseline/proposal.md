# Change: Add Application Manifest v1 and Ability Baseline

## Why

Macaca Application currently remains centered on legacy YAML `AppManifest`, which cannot express a long-term application ecosystem with GUI, headless, WASM, Store, Plugin-enhanced, and hybrid applications. Route C requires Application Framework semantics to live in the application layer while Kernel stays limited to system invariants.

## What Changes

- Add provider-neutral Application Manifest v1 contracts for package metadata, runtime profile, permissions, services, UI, commerce, plugin dependencies, and compatibility declarations.
- Add an Ability Descriptor model so one application can contain `AgentAbility`, `UiAbility`, `HeadlessAbility`, `ScheduledAbility`, `GatewayAbility`, and `ExtensionAbility`.
- Add admission/specification primitives for manifest, ability, permission, service, capability, trace, and compatibility validation.
- Keep legacy YAML `AppManifest` unchanged and prepare it to be adapted into Manifest v1 in a later proposal.
- Ensure manifest and ability DTOs live in protocol/foundation boundaries and do not depend on runtime-host, Web, Kernel, or provider implementations.

## Impact

- Affected specs: `application-manifest-v1`
- Affected code:
  - `macaca/crates/foundation/macaca-proto/src/application_manifest.rs`
  - `macaca/crates/foundation/macaca-proto/src/application_ability.rs`
  - `macaca/crates/foundation/macaca-proto/src/application_abi.rs`
  - `macaca/crates/application/macaca-app/src/manifest_v1/`
  - `macaca/crates/application/macaca-app/src/ability/`
  - `macaca/crates/application/macaca-app/src/compatibility_checker/`
  - `macaca/crates/application/macaca-app/src/lib.rs`
- Governance: must preserve Route C microkernel boundaries and introduce no new kernel/provider dependency.
