# Change: Add Application LLM Model Selection

## Why

Applications can currently send a `model` field through `/api/chat/v2`, but the
field is not consistently resolved through `service.llm` or exposed as a
backend-owned provider/model catalog. This makes Codex-class applications unable
to present a truthful model selector while preserving Macaca OS service
boundaries.

## What Changes

- Add an application-facing, service-owned LLM provider/model catalog path.
- Require model catalog, provider capability, and route resolution reads to flow
  through `service.llm` typed commands with trace context.
- Propagate request-level model/provider hints from application execution into
  service-owned route resolution before agent/app/system defaults.
- Allow app-owned UI bundles to call the catalog and route commands through the
  generic bridge when declared by manifest capabilities.
- Record sanitized route intent and effective route metadata for audit, replay,
  and diagnostics.

## Impact

- Affected specs: `llm-service`, `application-ui-runtime`
- Affected code: `macaca-llm`, `macaca-runtime-host`, `macaca-sdk`,
  `macaca-web`, `frontend`, and app-owned UI bridge consumers.
