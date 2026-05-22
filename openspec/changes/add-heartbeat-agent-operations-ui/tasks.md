# Tasks: Add Heartbeat Agent Operations UI

## 1. Contracts And SDK
- [x] 1.1 Add Heartbeat profile update command/result DTOs.
- [x] 1.2 Add Heartbeat service trait/client method for profile updates.
- [x] 1.3 Implement local provider profile updates with audit ids and logs.
- [x] 1.4 Add focused Heartbeat contract tests.

## 2. Web Routes
- [x] 2.1 Wire a focused Heartbeat SDK client into `AppState`.
- [x] 2.2 Add application-scoped Heartbeat operations snapshot route.
- [x] 2.3 Add application-scoped Heartbeat profile update route.
- [x] 2.4 Add route tests for sanitized aggregation and profile update mapping.

## 3. Frontend
- [x] 3.1 Add Heartbeat operations DTOs and API helpers.
- [x] 3.2 Add adjacent Scheduler/Heartbeat controls to the application operations dialog.
- [x] 3.3 Add Heartbeat panel, profile editor, agent list, profile list, and run timeline components.
- [x] 3.4 Preserve the existing terminal-style modal visual language.

## 4. Validation
- [x] 4.1 Run OpenSpec strict validation.
- [x] 4.2 Run Rust formatting, focused tests, and cargo checks.
- [x] 4.3 Run frontend lint and TypeScript checks.
- [x] 4.4 Run GitNexus detect changes and review affected flows.
