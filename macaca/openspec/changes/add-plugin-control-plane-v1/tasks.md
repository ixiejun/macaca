## 1. Preparation

- [x] 1.1 Read `docs/superpowers/plans/2026-05-11-plugin-service-enrichment-plan.md`.
- [x] 1.2 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-serviceization-allowlist.md`, and `macaca/docs/route-c-architecture-governance.md`.
- [x] 1.3 Read existing Plugin Runtime v0 code and OpenSpec change.
- [x] 1.4 Run GitNexus impact before editing any existing symbol and report blast radius.

## 2. Protocol Contracts

- [x] 2.1 Add plugin repository, install source, package location, install request/result, activation state, config schema, secret/env requirement, health snapshot, and diagnostics DTOs.
- [x] 2.2 Ensure DTOs are serde-friendly, provider-neutral, and do not expose secret values.
- [x] 2.3 Add detailed English comments explaining protocol invariants and security boundaries.

## 3. Runtime-Host Control Plane

- [x] 3.1 Add `PluginRepository` and install-source Strategy abstractions.
- [x] 3.2 Add manifest loader and compatibility policy.
- [x] 3.3 Add admission chain for manifest, signature metadata, compatibility, permissions, resources, config, secret, and entitlement placeholders.
- [x] 3.4 Add activation policy and deterministic health/diagnostics snapshots.
- [x] 3.5 Add `PluginControlService` facade with typed control commands.
- [x] 3.6 Emit structured logs and trace/audit records for every critical operation.

## 4. SDK And Shells

- [x] 4.1 Add SDK Plugin Control client.
- [x] 4.2 Add CLI management commands that use SDK/service client only.
- [x] 4.3 Add Web management routes that use SDK/service client only.
- [x] 4.4 Mark replaced direct control paths as deprecated compatibility anchors.

## 5. Documentation

- [x] 5.1 Update plugin development guide with control-plane lifecycle and management commands.
- [x] 5.2 Update governance docs only if new allowed dependency edges or rules are introduced.

## 6. Verification

- [x] 6.1 Run `openspec validate add-plugin-control-plane-v1 --strict`.
- [x] 6.2 Run `cargo fmt --all --check`.
- [x] 6.3 Run `cargo check --workspace`.
- [x] 6.4 Run `cargo test -p macaca-proto plugin`.
- [x] 6.5 Run `cargo test -p macaca-runtime-host plugin_control`.
- [x] 6.6 Run `cargo test -p macaca-sdk plugin_client`.
- [x] 6.7 Run `cargo test -p macaca-cli plugin`.
- [x] 6.8 Run `cargo test -p macaca-web plugin`.
- [x] 6.9 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 6.10 Run `npx gitnexus detect-changes -r agent` before commit.
