# Plugin Development Guide

Plugins extend Macaca OS system surfaces through service registry, permission, lifecycle, and trace contracts. A plugin must not bypass the service registry or call internal kernel/web implementation details.

## Supported Plugin Classes

- Gateway plugin: exposes external ingress capability through gateway service contracts.
- Driver plugin: exposes controlled real-world software or resource operations through driver service contracts.
- Skill/MCP plugin: exposes tool or protocol capability through skill/MCP service contracts.

## Required Metadata

- `package_type`: `plugin`, `driver`, `skill`, or `mcp`
- `runtime.kind`: `native_adapter`, `remote_service`, or `encrypted_text_bundle`
- lifecycle metadata such as `install`, `activate`, and `disable`
- required service declarations
- permission declarations with reasons
- provided capability declarations
- trace metadata

## Unavailable-Safe Behavior

A missing optional plugin dependency must become a structured warning. A missing required plugin service must become an incompatible certification report. Plugins must not block forever while waiting for a gateway, driver, browser, or MCP process.

## Trace And Audit

Plugin install, activation, disable, service registration, capability call, resource lock, and error paths must emit trace/audit events. Certification rejects or warns on metadata that cannot support transparent operation.

## Plugin Control Plane V1

Plugin Control Plane V1 manages plugin metadata, admission, activation state, health, diagnostics, and shell-facing inspection. It is a control plane only: it does not execute third-party code, launch WASM, spawn native processes, or call plugin capabilities directly.

### Management Boundary

- Web and CLI must use `macaca-sdk` Plugin Control clients.
- Runtime hosts must expose Plugin Control through `macaca-runtime-host::PluginControlSystemServiceProvider`.
- Kernel code must only keep identity, lifecycle, and descriptor ownership invariants.
- Plugin diagnostics must expose required config keys and secret names/status only. Diagnostics must never include secret values, API keys, private keys, raw package bytes, or full unbounded manifests.

### Supported Control Commands

- `plugin.list`: list installed plugin records.
- `plugin.inspect`: inspect one installed plugin.
- `plugin.install`: install a manifest-bearing package location into control-plane state.
- `plugin.enable`: register descriptor-safe plugin metadata with Plugin Runtime v0.
- `plugin.disable`: keep the record but prevent start.
- `plugin.start`: start descriptor-safe runtime state through Plugin Runtime v0.
- `plugin.stop`: stop descriptor-safe runtime state through Plugin Runtime v0.
- `plugin.uninstall`: remove the control record and runtime descriptors when cleanup is safe.
- `plugin.health`: return a deterministic health snapshot.
- `plugin.diagnostics`: return sanitized config/secret/admission diagnostics.

### Install Sources

The protocol supports bundled, user-local, project-local, dev-link, archive, store-cache, git, and custom source kinds. Project-local installs are disabled by default and require an explicit host policy opt-in. Archive, git, and store-backed loading are strategy extension points; V1 accepts explicit manifests and validates source policy before state mutation.

### CLI And Web Surfaces

The current CLI management entry is:

```bash
cargo run -p macaca-cli -- plugin list
```

The current Web management routes are:

```text
GET /api/plugins
GET /api/plugins/{id}
GET /api/plugins/{id}/diagnostics
```

## Certification

Gateway and driver plugin fixtures are certified by:

```bash
cargo test -p macaca-integration-tests package_certification
```

Certification validates metadata and unavailable-safe behavior without naming or requiring a real provider.
