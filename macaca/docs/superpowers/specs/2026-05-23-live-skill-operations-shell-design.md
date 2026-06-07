# Live Skill Operations Shell Design

## Context

The Web route and frontend panel already prove the `service.skill` read/write path can reach the live runtime. The remaining gap is shell consistency: CLI Skill commands currently instantiate `UnavailableSystemSkillClient`, so they report structured absence even when a live Web/runtime process has governance records.

## Approaches Considered

1. Bootstrap a second local `ServiceRuntime` inside CLI.
   - Rejected because it would create a separate in-memory runtime and would not observe the Web process state that operators are validating.
2. Link CLI directly to Web internals.
   - Rejected because CLI and Web are both shells; CLI must not depend on Web semantic state or route implementation internals.
3. Add a CLI HTTP adapter to the local Web API.
   - Selected. CLI remains a terminal adapter, Web remains the live runtime shell, and all skill semantics continue to live behind SDK/service boundaries.

## Design

CLI Skill commands gain an explicit runtime target: `--app-id` scopes the command to an application, and `--api-base` or `MACACA_API_BASE` points at the live Web API, defaulting to `http://127.0.0.1:3001`. When `--app-id` is present, CLI forwards commands to the existing Web API endpoints and prints the returned sanitized JSON with the Web trace id. When `--app-id` is absent, CLI keeps the structured unavailable diagnostic path so non-live invocations never fake success.

The frontend panel keeps the mutation trace from RUN/APPLY/ROLLBACK visible after its refresh, so the user can prove the button submitted a service command. The reload trace remains observable through the snapshot payload, but it no longer overwrites the last command trace.

## Testing

Unit tests cover CLI payload formation, app-scoped URL construction, no Web crate import, and mutation trace retention in frontend helper tests where practical. Live verification must start the real backend and frontend, seed or mutate Skill governance, run CLI against the same `app_id`, click RUN in the frontend with network instrumentation, and confirm records, curation refs, report refs, rollback refs, and command traces are real service outputs.
