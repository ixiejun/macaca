## Context
The default in-process WASM provider is intentionally small and engine-private. This change adds governance as composable policy and guard layers around that provider instead of baking limits directly into application-specific code.

## Goals / Non-Goals
- Goals: define stable policy DTOs, enforce payload and concurrency limits at runtime, emit sanitized audit reports, and keep WASI deny-by-default.
- Non-Goals: add a full WASI implementation, host import service portal, OS cgroups, process isolation, or a concrete third-party WASM engine dependency.

## Design Pattern Selection
- Strategy: resource, sandbox, and WASI policy DTOs can be interpreted by future engines without changing caller contracts.
- Decorator: runtime sessions gain guard behavior around dispatch while preserving the `WasmExecutionSession` trait.
- Specification: policy validation and merge rules are deterministic and testable before provider construction.
- Chain of Responsibility: dispatch first checks trace, then payload, then concurrency, then private engine invocation.
- Observer: each deny or exhaustion path creates a sanitized audit report and logs only stable identifiers and reason codes.

## Technical Decisions
Policy data lives in `macaca-proto` because Application Framework admission, SDK fixtures, and runtime providers need one shared vocabulary. Runtime enforcement lives in `macaca-runtime-host` because only the host sees concrete dispatch payloads and active sessions. The in-process adapter remains private and dependency-free; limits that cannot be faithfully enforced by the current minimal adapter are represented in policy and audit but fail closed only when requested behavior would expose raw WASI/env/fs/network access.

## Audit and Redaction
Audit reports include scope, reason code, trace id, runtime kind, and bounded metadata. They never include raw WASM bytes, raw command payloads, raw environment variables, filesystem paths, network destinations, secrets, prompts, private keys, or memory dumps.
