# Change: Migrate YAML Apps to Manifest v1 AgentAbility

## Why

YAML applications must remain first-class, but they must stop being the privileged center for new Application Platform capabilities. Existing YAML behavior should be preserved while the production application model moves to Manifest v1 and Ability descriptors.

## What Changes

- Add a YAML-to-Manifest-v1 adapter that projects legacy YAML `AppManifest` into `ApplicationManifestV1`.
- Map YAML agents, entrypoint, workflows, resources, context, skill policy, and tool policy into `AgentAbility` descriptors and sanitized compatibility reports.
- Update package and ABI descriptor generation to prefer Manifest v1 projections while keeping legacy paths as deprecated compatibility anchors.
- Preserve current YAML application loading, entry agent resolution, agent config resolution, workflow/resource/context behavior, and existing integration tests.

## Impact

- Affected specs: `application-yaml-adapter`
- Affected code:
  - `macaca/crates/application/macaca-app/src/model.rs`
  - `macaca/crates/application/macaca-app/src/loader.rs`
  - `macaca/crates/application/macaca-app/src/package.rs`
  - `macaca/crates/application/macaca-app/src/abi.rs`
  - `macaca/crates/application/macaca-app/src/runtime.rs`
  - `macaca/crates/application/macaca-app/src/manifest_v1/yaml_adapter.rs`
  - `macaca/crates/tests/macaca-integration-tests/tests/app_declarative.rs`
- Depends on: `add-application-manifest-v1-ability-baseline`
