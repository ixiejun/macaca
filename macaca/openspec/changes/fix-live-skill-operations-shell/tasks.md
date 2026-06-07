## 1. Specification

- [x] 1.1 Add shell-live Skill operations OpenSpec deltas.
- [x] 1.2 Validate `fix-live-skill-operations-shell` strictly.

## 2. CLI Live Adapter

- [x] 2.1 Add app-scoped live target arguments to Skill CLI commands.
- [x] 2.2 Add a public-Web-API adapter for operations, curation run/apply, lifecycle, rollback, and proposal decisions.
- [x] 2.3 Preserve the existing structured unavailable diagnostic path when no app id is supplied.
- [x] 2.4 Add tests proving URL construction, payload shaping, and no Web crate import.

## 3. Frontend Trace Observability

- [x] 3.1 Preserve mutation trace ids separately from refresh trace ids.
- [x] 3.2 Verify RUN emits a Web command trace and remains visible after refresh.

## 4. Verification

- [x] 4.1 Run OpenSpec validation.
- [x] 4.2 Run focused Rust and frontend checks.
- [x] 4.3 Run live backend/API/CLI/frontend e2e proof that the same app-scoped Skill governance state is visible and mutable through the live service path.
