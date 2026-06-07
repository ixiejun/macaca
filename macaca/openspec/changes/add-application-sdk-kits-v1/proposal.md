# Change: Add Application SDK Kits v1

## Why

Macaca needs a developer-facing SDK that lets application authors build real applications without importing internal runtime crates or hand-writing low-level DTOs. Existing `macaca-sdk` contains ABI helpers and shell-facing application clients, but it does not yet provide ApplicationKit, AbilityKit, Manifest Builder, Package Builder, or TestKit.

## What Changes

- Add `ApplicationKit` and `AbilityKit` builders over Application Manifest v1 and Ability Descriptor contracts.
- Add manifest, package, permission, service requirement, capability, GenUI surface, headless activation, and fixture builders.
- Add `ApplicationContractTestKit` for SDK-side validation of trace, permission, service, ability, runtime, and unsafe-payload rules.
- Add SDK examples/fixtures for declarative, GenUI, headless, plugin-enhanced, Store-entitled, and WASM skeleton applications.
- Keep `SystemApplicationClient` as shell-facing and separate from developer-facing SDK kits.

## Impact

- Affected specs: `application-sdk-kits`
- Affected code:
  - `macaca/crates/facade/macaca-sdk/src/application.rs`
  - `macaca/crates/facade/macaca-sdk/src/application_kit/`
  - `macaca/crates/facade/macaca-sdk/src/ability_kit/`
  - `macaca/crates/facade/macaca-sdk/src/application_testkit/`
  - `macaca/crates/facade/macaca-sdk/src/package_fixtures.rs`
  - `macaca/crates/facade/macaca-sdk/examples/`
  - `macaca/crates/facade/macaca-sdk/src/lib.rs`
- Depends on: `add-application-manifest-v1-ability-baseline`
