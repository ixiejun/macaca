## 1. Specification

- [x] 1.1 Add proposal, design, tasks, and spec deltas.
- [x] 1.2 Validate OpenSpec in strict mode.

## 2. Event Bridge

- [x] 2.1 Add a small `macaca-web` runtime event bridge helper.
- [x] 2.2 Verify helper writes EventLog before optional SSE.
- [x] 2.3 Keep helper payloads generic and sanitized.

## 3. Skill Events

- [x] 3.1 Emit skill snapshot cache/build/ready/failure events.
- [x] 3.2 Emit skill-backed MCP registration events through the shared helper.
- [x] 3.3 Add tests for sanitized skill event payloads.

## 4. Data Retrieval / Service-Call Events

- [x] 4.1 Convert session host-command result evidence into bounded session EventLog payloads.
- [x] 4.2 Bridge session-scoped data-result events to Web SSE after persistence.
- [x] 4.3 Add tests for sanitized service-call result payloads.

## 5. Validation

- [x] 5.1 Run `openspec validate add-observable-runtime-events --strict`.
- [x] 5.2 Run targeted `macaca-web` tests.
- [x] 5.3 Run `cargo check -p macaca-web`.
- [x] 5.4 Run GitNexus change detection.
