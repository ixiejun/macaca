# GenUI Development Guide

GenUI lets applications define their own UI surface through declarative UI schema and UI event commands. The application owns business behavior and UI intent; presentation shells render validated schema and forward events.

## Runtime Boundary

GenUI is an application framework capability, not a web-only feature. Applications emit UI intents with app id, session id, surface id, component tree, and trace context. Renderers may be Web, CLI, or future shells.

## Required Fields

- UI schema version metadata
- app id
- session id
- surface id
- component tree
- trace context
- permission or approval prompts when needed

## Safety Rules

GenUI v0 is declarative. Component props must not include script-like payloads or handler names. UI events are commands that must include trace context and must be validated before persistence or dispatch.

## Renderer Unavailable

If no renderer is installed, the host must return a structured unavailable result. Applications must not assume Web UI exists, and presentation shells must not define application semantics.

## Certification

Run:

```bash
cargo test -p macaca-integration-tests package_certification
```

The GenUI certification path checks schema metadata, trace requirements, and renderer-unavailable behavior without launching a frontend.
