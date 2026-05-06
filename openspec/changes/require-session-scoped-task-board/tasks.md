# Tasks

## 1. API guard

- [x] 1.1 Add a small helper that validates non-empty `session_id`.
- [x] 1.2 Update `list_todos` to return `400` when `session_id` is missing or blank.
- [x] 1.3 Add unit tests for the helper.

## 2. Front-end guard

- [x] 2.1 Make `fetchTodos` require a session id and encode it safely.
- [x] 2.2 Make `TaskBoardModal` show an error instead of issuing a request when no session id is available.

## 3. Verification

- [x] 3.1 Run OpenSpec validation.
- [x] 3.2 Run targeted Rust tests.
- [x] 3.3 Run frontend type/lint checks.
- [x] 3.4 Run GitNexus detect_changes.
- [x] 3.5 Commit backend/OpenSpec and frontend changes.
