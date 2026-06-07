# Change: Add Plugin Control Plane v1

## Why

Plugin Runtime v0 already defines manifest, lifecycle, descriptor registration, and kernel ownership invariants, but Macaca still lacks the control plane that makes plugins installable, inspectable, configurable, enableable, disableable, health-checkable, and manageable through service boundaries.

Without a control plane, plugins remain descriptor skeletons and Web/CLI would eventually be tempted to read plugin directories or manipulate runtime internals directly, violating Route C thin-shell and microkernel boundaries.

## What Changes

- Add provider-neutral Plugin Control Plane contracts for repositories, install sources, package locations, install requests/results, activation state, config schema, secret/env requirements, health snapshots, and diagnostics.
- Add a runtime-host-owned `PluginControlService` facade that coordinates repository discovery, manifest loading, admission, activation state, lifecycle handoff, health snapshots, and trace/audit events.
- Add command-driven service operations: `plugin.list`, `plugin.inspect`, `plugin.install`, `plugin.enable`, `plugin.disable`, `plugin.start`, `plugin.stop`, `plugin.uninstall`, `plugin.health`, and `plugin.diagnostics`.
- Add SDK client and Web/CLI thin-shell management surfaces that call Plugin Control Service instead of reading plugin storage or runtime internals.
- Keep install sources additive and safe: bundled, user-local, project-local opt-in, dev-link, archive, store-cache placeholder, and git placeholder.
- Preserve existing Plugin Runtime v0 APIs as compatibility anchors; mark newly bypassed direct paths as deprecated when a canonical control-plane API replaces them.

## Impact

- Affected specs: `plugin-control-plane`
- Affected code: `macaca-proto`, `macaca-runtime-host`, `macaca-sdk`, `macaca-web`, `macaca-cli`, `macaca-integration-tests`, `macaca/docs/developer/plugin-development-guide.md`
- Affected governance: `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-serviceization-allowlist.md`, `macaca/docs/route-c-architecture-governance.md`
- Affected tests: plugin proto/control-plane tests, SDK plugin client tests, Web/CLI plugin management tests, Route C dependency boundary tests

## Required Governance

- Kernel must not execute plugin code or own plugin storage.
- Web/CLI must remain thin shells and use SDK/service clients.
- Plugin control commands must require trace context and emit logs/audit.
- No provider, application, workflow, driver, gateway, model, chain, or business-specific hardcoding.
- New Rust code must include detailed English comments and logs at critical execution nodes.
