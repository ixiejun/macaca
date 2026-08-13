## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, the umbrella industrial catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API comparison notes for RFC 5545 recurrence, Temporal Schedules, Quartz triggers/misfires, and Airflow timetables/catch-up/backfill.
- [x] 1.3 Confirm boundaries with workflow task, approval, delegation, review, recovery, communication calendar, foundation time, location timezone, and shell rendering.
- [x] 1.4 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits, per the current refactor instruction.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define provider-neutral commands for create, update, pause, resume, delete, inspect, preview, next occurrences, fire due, backfill, cancel trigger, get history, and snapshot.
- [x] 2.2 Define `WorkflowSchedule`, `WorkflowScheduleSpec`, `ScheduleRecurrence`, `ScheduleTimezonePolicy`, `ScheduleMisfirePolicy`, `ScheduleOverlapPolicy`, `ScheduleTriggerRecord`, `ScheduleBackfillRequest`, and `WorkflowScheduleError`.
- [x] 2.3 Define typed success, partial, denied, unavailable, unsupported, invalid-recurrence, invalid-timezone, DST-unresolved, misfire-blocked, overlap-blocked, backfill-too-large, schedule-paused, trigger-conflict, quota-exceeded, provider-failure, and version-mismatch results.
- [x] 2.4 Define descriptor metadata for pack id, family, lifecycle, command schemas, recurrence support, cron support, timezone/DST support, misfire/catch-up/overlap policies, backfill limits, permission scopes, policy template, resource budgets, SDK metadata, compatibility, diagnostics, and documentation URL.
- [x] 2.5 Add stable descriptor hashing, version compatibility checks, DTO snapshot fixtures, recurrence fixtures, DST fixtures, misfire fixtures, backfill fixtures, and schema migration tests.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for `workflow.schedule.read`, `workflow.schedule.write`, `workflow.schedule.control`, `workflow.schedule.fire`, `workflow.schedule.backfill`, and `workflow.schedule.admin`.
- [x] 3.2 Enforce recurrence, timezone, DST strategy, action target, idempotency key derivation, misfire, catch-up, overlap, preview range, backfill range, and redaction policies before dispatch.
- [x] 3.3 Require every generated trigger to carry idempotency key, scheduled time, logical time, action reference, and policy evidence.
- [x] 3.4 Add resource reservation and quota checks for active schedules, pending triggers, backfill triggers, preview ranges, history size, retained snapshots, and replay metadata.
- [x] 3.5 Add approval behavior for large backfill, high-frequency schedules, critical action targets, catch-up floods, and administrative repair.
- [x] 3.6 Add tests proving denied, unavailable, invalid-recurrence, DST-unresolved, misfire-blocked, overlap-blocked, backfill-too-large, paused, trigger-conflict, and quota paths do not create tasks or service commands incorrectly.

## 4. Service Provider And Recurrence Strategy

- [x] 4.1 Implement the workflow schedule service provider contract behind the service runtime; do not construct providers from kernel, SDK, shells, or generic application-framework code.
- [x] 4.2 Add provider descriptor support for built-in durable, remote workflow-engine, plugin, mock, and unavailable provider classes.
- [x] 4.3 Add schedule, trigger, pause/resume, backfill, misfire, catch-up, overlap, and deletion state machines.
- [x] 4.4 Add mock and unavailable providers for deterministic tests; external schedule adapters must remain optional providers or plugin/remote modules.
- [x] 4.5 Add provider conformance tests for one-shot, interval, cron, RRULE, inclusions/exclusions, timezone/DST, preview, fire due, misfire, catch-up, backfill, overlap, history, snapshot, redaction, and unsupported-command reporting.
- [x] 4.6 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, trigger idempotency, resource cleanup, and bounded output behavior.

## 5. SDK, Admission, Examples, And ABI

- [x] 5.1 Extend SDK discovery for `pack.workflow.schedule.v1` with command schemas, DTO schemas, permission scopes, examples, availability, recurrence support, timezone/DST support, misfire/catch-up/overlap policies, diagnostics, compatibility, and documentation URL.
- [x] 5.2 Extend application admission so required declarations block when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders that only produce canonical traced service calls and never construct providers or branch on schedule/task/application names.
- [x] 5.4 Add WASM/application ABI exposure for schedule commands using provider-neutral DTO schemas and canonical service-call dispatch.
- [x] 5.5 Add generic examples for one-shot, interval, cron, RRULE, preview, pause/resume, fire due, backfill, misfire, overlap, task integration, history, and unavailable-provider diagnostics.

## 6. Trace, Audit, Replay, And Boundary Gates

- [x] 6.1 Emit sanitized schedule lifecycle events for declaration, admission, creation, update, pause, resume, preview, trigger computation, trigger fire, trigger cancellation, backfill start/completion, misfire handling, deletion, and snapshot recording.
- [x] 6.2 Add replay tests proving every computed/fired trigger is trace-addressable through the canonical service path after refresh/restart.
- [x] 6.3 Add dependency-boundary gates proving microkernel, SDK, shells, and generic application framework do not import concrete schedule providers or recurrence engines directly.
- [x] 6.4 Add no-direct-provider-call gates proving all schedule commands enter through descriptor-owned service registrations and typed service runtime dispatch.
- [x] 6.5 Add redaction tests for action payloads, prompts, provider payloads, schedule metadata, trigger history, snapshots, and logs.
- [x] 6.6 Run `openspec validate add-pack-workflow-schedule --strict`, DTO compatibility tests, recurrence tests, DST tests, misfire/catch-up tests, backfill tests, boundary gates, file-size gates, and audit replay checks before marking implementation tasks complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/workflow/schedule.md` with purpose, manifest declarations, scopes, command DTOs, result DTOs, one-shot/interval/cron/RRULE schedules, timezone/DST behavior, preview, pause/resume, misfire/catch-up/backfill, overlap, task integration, history, unavailable diagnostics, and trace/audit behavior.
- [x] 7.2 Add provider author documentation covering descriptor fields, recurrence evaluator responsibilities, schedule/trigger/backfill state machines, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy.
- [x] 7.3 Add minimal app-facing examples for create/preview/fire/pause/resume/backfill/history using generic synthetic data.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-workflow-schedule` complete.
