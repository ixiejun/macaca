# Superpowers Brainstorm + Write-Plan: Workbench Shell Adapters

Date: 2026-05-28

## Brainstorm

Goal: implement section 17 of `complete-codex-class-application-support` without
turning Web, CLI, or frontend into semantic owners.

Options considered:

- Add separate Web routes for every operation family. This gives many endpoints
  but risks duplicating service command semantics in the shell.
- Add a single diagnostics adapter over focused workbench clients. This keeps
  shells thin, exercises every service descriptor/snapshot path, and gives
  frontend/CLI one stable operator surface.
- Add frontend-only static panels. This is rejected because it would be
  descriptor-only and would not prove service-backed degradation.

Selected approach: a data-driven diagnostics adapter using Facade,
Adapter/Bridge, Observer, Null Object, and Memento patterns. The route calls the
focused SDK clients and returns descriptors, snapshots, health, trace refs, and
adapter endpoint coverage only. It does not define policy, approval decisions,
tool routing, plugin lifecycle, filesystem semantics, process semantics, or
application workflow behavior.

Risks:

- Some providers may be absent or not yet bootstrapped in Web. The adapter must
  expose structured unavailable/error rows rather than failing the whole route.
- CLI must not import runtime-host composition roots. It can use SDK Null Object
  clients locally or call the already-running Web facade when an app id is
  supplied.
- Frontend must keep the surface compact and data-dense, with graceful loading
  and failure states.

## Write-Plan

- Update OpenSpec delta to state that shell adapters are renderer/transport
  surfaces only.
- Add a `workbench_routes` module with an app-scoped operations endpoint that
  builds focused clients over Web's generic service dispatcher.
- Register the route in the Axum router and add source-level thin-shell tests.
- Add CLI `workbench operations` command with SDK Null Object fallback and
  optional live Web API target.
- Add frontend typed fetch helpers and a workbench operations panel under the
  application operations dialog.
- Validate OpenSpec, targeted Rust tests, frontend lint/build, and run
  GitNexus detect changes before committing.
