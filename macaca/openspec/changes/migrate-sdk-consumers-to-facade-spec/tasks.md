## 1. Preparation

- [x] 1.1 Read SDK refactor plan, SDK code, and upper consumer call sites.
- [x] 1.2 Run GitNexus impact for `register_from_config` and `build_with_manifest`.
- [x] 1.3 Refresh stale GitNexus index.

## 2. OpenSpec

- [x] 2.1 Create proposal, design, tasks, and delta spec.
- [x] 2.2 Validate `migrate-sdk-consumers-to-facade-spec` with `--strict`.

## 3. Production Migration

- [x] 3.1 Migrate `macaca-app` runtime registration to `MacacaSdk::for_kernel(...).register_config(...)`.
- [x] 3.2 Confirm `macaca-app` no longer imports deprecated SDK registry helpers.

## 4. Test Migration

- [x] 4.1 Migrate integration lifecycle tests from `build_with_manifest` to `build_spec`.
- [x] 4.2 Migrate ignored live integration test from `build_with_manifest` to `build_spec`.
- [x] 4.3 Migrate kernel e2e tests from `build_with_manifest` to a spec-based helper.
- [x] 4.4 Migrate upper registration tests from deprecated helpers to `MacacaSdk`.

## 5. Verification

- [x] 5.1 Scan upper consumers for deprecated SDK calls.
- [x] 5.2 Run focused SDK/app/kernel/integration checks.
- [x] 5.3 Run `openspec validate migrate-sdk-consumers-to-facade-spec --strict`.
- [x] 5.4 Run GitNexus detect-changes and review affected scope.
