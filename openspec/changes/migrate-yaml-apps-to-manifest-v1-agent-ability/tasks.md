## 1. YAML Adapter
- [x] 1.1 Add `YamlApplicationManifestAdapter`.
- [x] 1.2 Add `LegacyAppManifestProjection`.
- [x] 1.3 Add `YamlToApplicationManifestV1Report`.
- [x] 1.4 Map YAML agents to `AgentAbility` descriptors.

## 2. Descriptor Migration
- [x] 2.1 Update package descriptor generation to prefer Manifest v1 projection.
- [x] 2.2 Update ABI descriptor generation to preserve key metadata through Manifest v1.
- [x] 2.3 Mark legacy-only direct descriptor helpers as deprecated where appropriate.

## 3. Compatibility Tests
- [x] 3.1 Add tests proving existing YAML app behavior remains unchanged.
- [x] 3.2 Add tests proving YAML projection creates at least one AgentAbility.
- [x] 3.3 Add tests comparing legacy and projected package/ABI key fields.
- [x] 3.4 Run `cargo test -p macaca-app yaml`.
- [x] 3.5 Run `cargo test -p macaca-integration-tests --test app_declarative`.
- [x] 3.6 Run `cargo check --workspace`.
