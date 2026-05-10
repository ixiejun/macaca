# Macaca Route C Workspace Topology

This directory contains the Rust workspace crates grouped by Route C ownership
layer. The grouping is a filesystem signal for maintainers: it makes the
microkernel, serviceized capabilities, runtime host, application framework,
facade, shells, and cross-layer tests visible before reading crate internals.

Filesystem placement does not grant dependency permission. The executable Route
C dependency boundary gate remains the authority for forbidden crate edges and
allowlisted migration debt.

## Layers

| Layer | Crates | Ownership |
| --- | --- | --- |
| `foundation` | `macaca-proto`, `macaca-ipc`, `macaca-persist` | Shared protocol/ABI/service DTOs, service bus/transport bridge, and persistence foundation. |
| `kernel` | `macaca-kernel` | Microkernel invariants such as registry, scheduler, policy, trace, task, and session primitives. |
| `services` | `macaca-task`, `macaca-llm`, `macaca-memory`, `macaca-context`, `macaca-driver`, `macaca-skill`, `macaca-gateway`, `macaca-tools` | Replaceable system service domains and provider-neutral capability surfaces. |
| `runtime` | `macaca-runtime`, `macaca-runtime-host`, `macaca-framework` | Agentic runtime primitives, host-owned service lifecycle, provider wrappers, and traced framework seams. |
| `application` | `macaca-agent`, `macaca-app` | Agent primitives and Application Framework/package/lifecycle ownership. |
| `facade` | `macaca-sdk` | Shell/developer-facing SystemFacade and focused clients. |
| `shells` | `macaca-web`, `macaca-cli` | HTTP/Web/GenUI/trace viewer and terminal adapters. |
| `tests` | `macaca-integration-tests` | Cross-layer governance and regression tests. |

## Old-To-New Path Map

| Old path | New path |
| --- | --- |
| `crates/macaca-proto` | `crates/foundation/macaca-proto` |
| `crates/macaca-ipc` | `crates/foundation/macaca-ipc` |
| `crates/macaca-persist` | `crates/foundation/macaca-persist` |
| `crates/macaca-kernel` | `crates/kernel/macaca-kernel` |
| `crates/macaca-task` | `crates/services/macaca-task` |
| `crates/macaca-llm` | `crates/services/macaca-llm` |
| `crates/macaca-memory` | `crates/services/macaca-memory` |
| `crates/macaca-context` | `crates/services/macaca-context` |
| `crates/macaca-driver` | `crates/services/macaca-driver` |
| `crates/macaca-skill` | `crates/services/macaca-skill` |
| `crates/macaca-gateway` | `crates/services/macaca-gateway` |
| `crates/macaca-tools` | `crates/services/macaca-tools` |
| `crates/macaca-runtime` | `crates/runtime/macaca-runtime` |
| `crates/macaca-runtime-host` | `crates/runtime/macaca-runtime-host` |
| `crates/macaca-framework` | `crates/runtime/macaca-framework` |
| `crates/macaca-agent` | `crates/application/macaca-agent` |
| `crates/macaca-app` | `crates/application/macaca-app` |
| `crates/macaca-sdk` | `crates/facade/macaca-sdk` |
| `crates/macaca-web` | `crates/shells/macaca-web` |
| `crates/macaca-cli` | `crates/shells/macaca-cli` |
| `crates/macaca-integration-tests` | `crates/tests/macaca-integration-tests` |

## Maintenance Rules

- Keep package names and Rust crate names stable unless a separate OpenSpec
  proposal explicitly approves a rename.
- Add every new workspace crate to a Route C layer through OpenSpec before
  relying on it in production code.
- Prefer `cargo metadata` or layer-aware globs in executable scripts and tests
  instead of hardcoded flat paths such as `crates/macaca-web`.
- Historical research and proposal documents may still mention old flat paths;
  this README is the current topology source of truth.
