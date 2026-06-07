## 1. Preparation

- [x] 1.1 Run GitNexus impact for `Kernel` upstream.
- [x] 1.2 Run GitNexus impact for `run_kernel` upstream before editing CLI startup.
- [x] 1.3 Run GitNexus impact for `start_server` upstream before editing web startup.
- [x] 1.4 Run GitNexus impact for `SimpleScheduler` upstream before editing scheduler tests.
- [x] 1.5 Confirm current deprecated kernel consumer calls with grep.

## 2. OpenSpec validation

- [x] 2.1 Run `openspec validate migrate-kernel-consumers-to-builder --strict`.

## 3. Production consumer migration

- [x] 3.1 Migrate `macaca-web/src/lib.rs` to `KernelBuilder`.
- [x] 3.2 Migrate `macaca-cli/src/commands.rs` to a local builder-backed helper.
- [x] 3.3 Migrate `macaca-app` helper construction to `KernelBuilder`.
- [x] 3.4 Migrate `macaca-sdk` helper construction to `KernelBuilder`.

## 4. Test consumer migration

- [x] 4.1 Migrate integration test kernel helpers to `KernelBuilder`.
- [x] 4.2 Migrate kernel e2e helper to `KernelBuilder`.
- [x] 4.3 Migrate direct scheduler test usage to `SchedulerFactory`.

## 5. Deprecated-call containment checks

- [x] 5.1 Verify production upper crates have no `Kernel::new` calls.
- [x] 5.2 Verify production upper crates have no direct `SimpleScheduler` calls.
- [x] 5.3 Verify all existing consumer call sites were migrated.

## 6. Verification

- [x] 6.1 Run `cargo fmt`.
- [x] 6.2 Run `cargo test -p macaca-kernel -- --nocapture`.
- [x] 6.3 Run `cargo test -p macaca-integration-tests kernel -- --nocapture`.
- [x] 6.4 Run `cargo check -p macaca-kernel -p macaca-web -p macaca-app -p macaca-sdk -p macaca-cli`.
- [x] 6.5 Run `openspec validate migrate-kernel-consumers-to-builder --strict`.
- [x] 6.6 Run `gitnexus_detect_changes(scope: "all")`.
