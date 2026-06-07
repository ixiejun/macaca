## 1. SDK Kits
- [x] 1.1 Add `ApplicationKit` facade.
- [x] 1.2 Add `AbilityKit` facade.
- [x] 1.3 Add Manifest v1, ability, permission, service, capability, GenUI surface, commerce, and plugin dependency builders.
- [x] 1.4 Keep `SystemApplicationClient` separate and shell-facing.

## 2. TestKit and Fixtures
- [x] 2.1 Add `ApplicationContractTestKit`.
- [x] 2.2 Add reusable application fixture builders.
- [x] 2.3 Add declarative, GenUI, headless, plugin-enhanced, Store-entitled, and WASM skeleton examples.

## 3. Validation
- [x] 3.1 Add unit tests for deterministic builder output.
- [x] 3.2 Add tests rejecting missing permissions, services, trace, and ability entries.
- [x] 3.3 Run `cargo test -p macaca-sdk application_kit`.
- [x] 3.4 Run `cargo test -p macaca-sdk application_testkit`.
- [x] 3.5 Run `cargo check -p macaca-sdk`.
- [x] 3.6 Run Route C dependency boundary tests.
