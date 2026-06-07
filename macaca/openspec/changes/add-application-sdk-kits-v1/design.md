## Context

The Application Platform requires an SDK that resembles platform SDKs from operating systems and mini-program ecosystems: high-level developer APIs backed by stable provider-neutral contracts. SDK code must not construct application runtime, kernel, runtime-host providers, Web state, or service runtime.

## Goals

- Provide ergonomic builders for Manifest v1 and Ability descriptors.
- Provide a contract test kit for application packages and fixtures.
- Keep developer SDK separate from shell-facing `SystemApplicationClient`.
- Support YAML-equivalent, GenUI, headless, plugin-enhanced, Store-entitled, and WASM skeleton fixtures.

## Non-Goals

- Do not execute applications.
- Do not construct `AppRuntime`, `Kernel`, `ServiceRuntime`, Web state, or runtime-host providers.
- Do not implement CLI commands in this proposal.
- Do not migrate YAML consumers.

## Decisions

- Decision: Use Builder for all developer-authored manifests and abilities.
  Rationale: builders reduce low-level DTO mistakes while still returning serializable provider-neutral data.

- Decision: Use Facade for `ApplicationKit` and `AbilityKit`.
  Rationale: developers need stable entry points that hide internal module layout.

- Decision: Use Specification and Visitor in `ApplicationContractTestKit`.
  Rationale: the test kit must walk manifest/ability declarations and report precise violations without executing runtime code.

- Decision: Keep shell client and developer SDK separate.
  Rationale: `SystemApplicationClient` is for Web/CLI/Gateway controlling Application Service; `ApplicationKit` is for app authors declaring application packages.

- Decision: Use Null Object expectations in fixtures.
  Rationale: WASM, Store, Plugin, and optional service examples must be testable without real external providers.

## Risks / Trade-offs

- Risk: SDK becomes a god facade.
  Mitigation: split into `application_kit`, `ability_kit`, `application_testkit`, and package fixture modules.

- Risk: SDK accidentally imports runtime internals.
  Mitigation: enforce dependency boundaries and keep SDK builders based on `macaca-proto`.

- Risk: Examples become business-specific.
  Mitigation: use generic fixture ids and avoid workflow/app/provider/business hardcoding.

## Migration Plan

1. Add SDK kit modules and builder types.
2. Add contract test kit over Manifest v1 contracts.
3. Add generic examples/fixtures.
4. Verify SDK crate and integration fixture tests.

## Trace / Audit

SDK-built host commands and fixture validations must carry trace where required. SDK logs may include operation, fixture id, ability kind, and reason code, but not prompt body, raw secrets, raw manifest body, env, or host payload.
