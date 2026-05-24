# Change: Fix Skill Telemetry Replay And API-First Audit

## Why

Live self-evolution proof showed that governed Skill usage counters increment in the current process, but reset after restart because the local Skill provider only stores governance events in memory. It also showed that agent-authored audit artifacts can contradict canonical platform state when they inspect filesystem evidence before service APIs.

## What Changes

- Add a durable local append-only journal for sanitized Skill governance events and replay it on Skill provider startup.
- Replay journal events before materialized package recovery so historical usage counters survive restart while package recovery still fills identity gaps.
- Add a canonical API-first self-evolution audit/trigger verification surface that reports operations, registry/load-path visibility, and session observer evidence before filesystem support evidence.

## Impact

- Affected specs: `skill-governance-curation`
- Affected code: `macaca-runtime-host` Skill provider, `macaca-web` Skill diagnostic route/adapters, self-evolution evidence ledger
