## 1. Specification

- [x] 1.1 Add autonomous-runtime requirements for nonblocking heartbeat dispatch and evidence-gated completion.
- [x] 1.2 Add serviceization escape-hatch requirements rejecting fake autonomy success and scheduler-blocking heartbeat dispatch.
- [x] 1.3 Validate the OpenSpec change strictly before claiming the spec is aligned.

## 2. Runtime Tests

- [x] 2.1 Add a heartbeat lane regression test where a slow Agent Execution backend must not block `tick_once`.
- [x] 2.2 Add a scheduled-agent dispatch regression test where completed Agent Execution without evidence becomes retryable.
- [x] 2.3 Update existing dispatch tests to provide explicit sanitized evidence when they expect success.
- [x] 2.4 Add regression tests proving `result_output_hash` alone is not completion evidence, unexpected artifact paths do not satisfy expected artifact metadata, and stale expected artifacts remain evidence-missing.

## 3. Implementation

- [x] 3.1 Introduce a provider-neutral Agent Execution evidence gate in runtime-host dispatch code.
- [x] 3.2 Apply the evidence gate to scheduled-agent Scheduler target dispatch before marking runs succeeded.
- [x] 3.3 Move heartbeat agent dispatch to bounded background tasks spawned by HeartbeatLane.
- [x] 3.4 Add key logs for heartbeat dispatch spawn/completion and evidence-gate decisions.
- [x] 3.5 Extract Agent Execution artifact evidence through a bounded Observer/Memento helper, capture expected-artifact file snapshots, and propagate generic `evidence.*` metadata from heartbeat and scheduled-agent declarations.

## 4. Verification

- [x] 4.1 Run targeted runtime-host autonomy tests.
- [x] 4.2 Run scheduled-agent service and integration boundary tests.
- [x] 4.3 Run `openspec validate fix-autonomy-dispatch-evidence --strict`.
- [x] 4.4 Run `git diff --check`.
- [x] 4.5 Run `gitnexus detect_changes --scope all` and review scope.
