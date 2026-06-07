# Change: Refactor macaca-memory with design-pattern primitives

## Why

`macaca-memory` currently exposes useful storage traits, but manager orchestration directly combines session, file, vector, and embedding behavior. The duplication makes future session resume, backend selection, embedding cache, and query strategy work harder to evolve safely.

## What Changes

- Add facade request/result helpers for manager-level remember/recall/forget flows.
- Add an in-process embedding cache and cached embedding decorator.
- Add backend factory config for standard memory manager construction.
- Add memory snapshot/replay memento types.
- Add vector query strategy primitives for current similarity search and future filtered/hybrid search.
- Mark superseded manager-level direct methods as deprecated while keeping them callable for compatibility and migration discovery.

## Impact

- Affected specs: `macaca-memory-core`
- Affected code: `macaca-memory`
- Compatibility: existing public store/provider/vector traits remain available; deprecated manager methods remain callable.
- Non-impact: no changes to `macaca-kernel`, `macaca-web`, `macaca-agent`, `macaca-framework`, task scheduling, trace, session, driver, skill, or MCP behavior.
