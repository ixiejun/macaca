## Context

Manifest-declared heartbeat agents now execute through native Heartbeat cadence and Agent Execution, but the Web UI only exposes Scheduler-oriented controls. The new surface must make heartbeat execution visible and editable without reintroducing Scheduler-owned heartbeat semantics.

## Goals

- Show manifest-declared heartbeat agents for the selected application.
- Show native heartbeat profile summaries and recent heartbeat run mementos.
- Allow safe profile edits for enabled state, fixed interval cadence, and metadata.
- Keep Web/frontend as shell adapters over SDK/service commands.
- Keep responses sanitized and bounded.

## Non-Goals

- Editing raw manifests or `HEARTBEAT.md`.
- Rendering raw prompts, provider payloads, package bytes, or unbounded output.
- Creating hidden Scheduler jobs for heartbeat.
- Encoding application-specific agent names, workflow names, or business semantics in OS/Web code.

## Decisions

- **Facade:** Web calls a focused `SystemHeartbeatClient` and Application Service, not providers.
- **Command:** Profile edits use a typed trace-bearing Heartbeat command.
- **Memento:** Snapshot and run history are returned as bounded mementos.
- **Observer:** Routes and providers log trace/audit-relevant execution nodes.
- **State:** Profile enabled/cadence state is explicit and independent from Scheduler job lifecycle.

## Risks And Mitigations

- **Risk:** UI appears to edit manifest declarations.
  **Mitigation:** The UI labels edits as native profile runtime policy and renders manifest declarations as read-only projections.

- **Risk:** Heartbeat becomes a Scheduler target again.
  **Mitigation:** No route creates Scheduler jobs; all heartbeat operations call `service.heartbeat`.

- **Risk:** Sensitive content leaks into operations responses.
  **Mitigation:** Web aggregates sanitized Application Service views and Heartbeat snapshots only.
