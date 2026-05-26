## 1. Specification

- [x] 1.1 Add OpenSpec proposal, design, tasks, and memory runtime persistence delta.
- [x] 1.2 Validate the OpenSpec change strictly.

## 2. Configured Memory Runtime

- [x] 2.1 Add failing tests for configuration-driven Milvus/DashScope backend selection.
- [x] 2.2 Implement trait-object vector and embedding dispatch adapters.
- [x] 2.3 Implement a configuration-driven memory manager factory.
- [x] 2.4 Wire `macaca-web` to use the configured factory instead of `test_manager()`.

## 3. Session Completion Capture

- [x] 3.1 Add failing tests for successful session capture through `SystemMemoryClient::remember`.
- [x] 3.2 Add a generic web session-memory capture adapter with bounded content and sanitized logs.
- [x] 3.3 Call the adapter after successful `/api/chat/v2` service-agent completion.

## 4. Verification

- [x] 4.1 Run targeted memory and web tests.
- [x] 4.2 Run OpenSpec validation and diff checks.
- [x] 4.3 Record GitNexus HIGH/CRITICAL output as non-blocking risk notes.
- [x] 4.4 Commit all frontend/backend changes in the repo.
- [x] 4.5 Verify composer active-recall timeout degradation and add a bounded Web-scoped fallback test.
