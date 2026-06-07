## Context

Web already bootstraps the live service runtime and exposes `service.skill` through SDK-backed routes. CLI is a separate shell process, so constructing its own runtime would not observe Web's live in-memory governance state and would risk duplicating provider composition.

## Decisions

- Use the Adapter pattern in CLI: an app-scoped HTTP adapter calls the existing Web API facade.
- Use the Command pattern at the shell boundary: CLI normalizes operator refs into the same request body fields that Web routes already translate into service DTOs.
- Use the Observer pattern for verification: both CLI logs and frontend UI preserve trace ids for command execution.
- Keep Null Object unavailable behavior when no app id is supplied, because a non-targeted CLI command must not fake live runtime success.

## Risks And Mitigations

- Risk: CLI could become coupled to Web internals.
  - Mitigation: depend only on public HTTP paths and do not import `macaca-web`.
- Risk: app-scoped Web routes require a live backend.
  - Mitigation: return structured config errors when the local API is unreachable.
- Risk: frontend refresh can hide the mutation trace.
  - Mitigation: store command trace separately from snapshot trace.

## Non-Goals

- Do not move Skill semantics into CLI or frontend.
- Do not add application-specific Skill behavior.
- Do not require Web to be present for diagnostic-only CLI commands.
