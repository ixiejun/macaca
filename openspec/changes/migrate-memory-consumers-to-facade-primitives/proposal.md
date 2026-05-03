# Change: Migrate memory consumers to facade primitives

## Why

`macaca-memory` now exposes facade request/result primitives (`RememberText`, `RecallQuery`, `RecallResult`, `ForgetMemory`) and backend construction helpers, but upper crates still expose and adapt memory through raw `store(MemoryEntry)` / `retrieve(&str, usize)` APIs.

This keeps new code coupled to pre-refactor memory semantics and makes it easy for future agent/service code to bypass the canonical memory facade.

## What Changes

- Add facade-first memory methods to `macaca-agent::MemoryService`.
- Mark old agent memory service `store` / `retrieve` methods as deprecated compatibility helpers.
- Migrate `NoopMemoryService`, tests, and service consumers to `remember_text` / `recall`.
- Migrate `macaca-kernel::MemoryServiceAdapter` to adapt facade-capable `macaca-memory` managers.
- Preserve old callable interfaces for compatibility and migration discovery.

## Impact

- Affected specs: `macaca-memory-consumer-migration`
- Affected code: `macaca-agent`, `macaca-kernel`
- Compatibility: old methods remain callable and grepable through deprecation markers.
- Non-impact: no changes to planner/worker loops, trace/session persistence, framework working memory, task scheduling, driver, skill, MCP, application-specific behavior, or user-visible workflow semantics.
