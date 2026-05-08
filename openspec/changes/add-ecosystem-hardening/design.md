# Design: Route C Ecosystem Hardening

## Context

Phase 13 is the final Route C ecosystem readiness layer. Earlier phases introduced kernel boundaries, system services, IPC, package manifests, Application ABI, GenUI, plugins, Store/Entitlement, A2A payment, optional Web3/EVM, and Web/CLI thin-shell contracts. Ecosystem hardening must prove those pieces can be consumed by third-party developers in a repeatable way.

This change is intentionally checker-first. Documentation and examples are necessary, but they are not enough. Every major developer path must be represented by package fixtures and certification tests so the ecosystem surface is executable, traceable, and auditable.

## Goals

- Provide developer-facing guidance for building applications, plugins, GenUI surfaces, Store-submitted packages, skills, MCP integrations, Web3 apps, and DApps.
- Provide SDK examples that can be read by automated compatibility checks.
- Provide a compatibility checker that reports `compatible`, `warning`, or `incompatible` diagnostics with stable codes.
- Provide certification tests for package classes without requiring real external services, real payment settlement, real WASM execution, or real blockchain nodes.
- Preserve all existing YAML app, `/api/chat/v2`, trace, task board, resume, driver, skill/MCP, and optional Web3/EVM behaviors.

## Non-Goals

- No real Store backend, payment settlement, encrypted package delivery service, blockchain node, EVM runtime, or WASM executor.
- No migration of presentation shell semantics.
- No removal of current YAML compatibility paths.
- No hardcoded application, provider, driver, gateway, chain, model, or demo-specific routing.

## Architecture

The implementation SHALL use these design patterns:

- `Specification`: each compatibility rule is an isolated predicate with a stable diagnostic code.
- `Visitor`: the checker walks package manifest, ABI metadata, capabilities, permissions, commerce metadata, optional modules, and documentation/example metadata without coupling checks to concrete package classes.
- `Facade`: SDK examples and certification tests call one public checker facade instead of internal guard steps.
- `Builder`: package fixtures construct valid/invalid descriptors without duplicating raw struct construction.
- `Template Method`: certification tests share a common package-class certification flow while each package type provides its own fixture inputs.
- `Observer`: checker decisions emit presentation-neutral trace/audit records and structured logs.

## Components

### Developer Documentation

Documentation lives under `macaca/docs/developer/` and is treated as a developer contract, not marketing copy. Each guide must include:

- package type and runtime kind;
- manifest fields;
- permissions and capability declarations;
- trace/audit expectations;
- debugging/certification commands;
- unavailable-safe behavior for optional services;
- Store/Entitlement implications where relevant.

### SDK Examples

Examples live under `macaca/crates/macaca-sdk/examples/`. They provide package fixtures for certification:

- YAML app fixture.
- WASM-stub app fixture.
- GenUI app fixture.
- gateway plugin fixture.
- driver plugin fixture.
- paid skill fixture.
- Web3 optional app fixture.
- EVM optional DApp fixture.

Examples must be data-driven and generic. They must not depend on real external LLMs, real browsers, real networks, real payment gateways, real Store servers, real blockchain nodes, or real EVM execution.

### Compatibility Checker

The checker lives in `macaca/crates/macaca-app/src/compatibility_checker.rs`. It must be additive and presentation-neutral. It should accept package descriptors and host context, then produce a report containing:

- package id and package type;
- manifest version;
- runtime kind and ABI version;
- compatibility status;
- diagnostics with stable codes, severity, message, and field path;
- trace/audit events;
- optional module status;
- upgrade compatibility notes.

The checker reuses existing package/runtime guard concepts but does not replace the runtime guard. Runtime guard answers whether a package can proceed to loading; compatibility checker answers whether a package is ecosystem-certifiable and why.

### Certification Tests

Certification tests live in `macaca/crates/macaca-integration-tests/tests/package_certification.rs`. They must certify each required package path:

- valid YAML app is compatible;
- WASM stub package is metadata-compatible but execution-unavailable;
- GenUI fixture validates trace and UI schema;
- gateway/driver plugin fixtures declare service/capability/permission metadata;
- paid skill fixture distinguishes entitlement missing and entitlement allowed;
- Web3 optional fixture passes when optional module is unavailable;
- EVM optional DApp fixture passes when optional module is unavailable;
- invalid fixtures produce structured diagnostics rather than panic, hang, or silent pass.

## Trace And Audit

Every checker run must create traceable records for:

- checker start;
- each rule start/pass/warn/fail;
- final decision;
- optional module unavailable;
- entitlement unavailable/denied/allowed;
- upgrade compatibility warning or rejection.

Implementation code must include detailed English comments explaining code intent and mechanics. Key decision points must log with structured `tracing` fields.

## Policy And Security

The checker must not grant permissions. It only validates declarations and reports whether a package is compatible with the host context. Capability calls remain subject to the canonical policy layer. Paid package certification must model entitlement allow/deny using existing Store/Entitlement contracts without implementing a real marketplace.

## Compatibility And Rollback

This change is additive-first:

1. Add docs and examples.
2. Add checker data model and rules.
3. Add tests.
4. Export the checker from `macaca-app`.
5. Extend baseline documentation/system overview.

Rollback is deleting the additive checker, examples, docs, and tests. Existing runtime behavior should remain untouched.

## Risks And Mitigations

- Risk: checker duplicates runtime guard logic. Mitigation: checker consumes package descriptors and host context, while runtime guard remains the pre-load enforcement path.
- Risk: docs drift from executable rules. Mitigation: certification tests must read SDK fixtures and verify every documented package class.
- Risk: examples become demo-specific. Mitigation: examples must use generic package types and capabilities, with hardcode scans over app/provider/driver/gateway/chain names.
- Risk: optional Web3/EVM accidentally becomes required. Mitigation: tests must assert unavailable-safe behavior.
