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

## Certification

Gateway and driver plugin fixtures are certified by:

```bash
cargo test -p macaca-integration-tests package_certification
```

Certification validates metadata and unavailable-safe behavior without naming or requiring a real provider.
