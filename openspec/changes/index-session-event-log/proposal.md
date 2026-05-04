# Change: Index session event log reads

## Why

Filtered session event reads currently replay session events through storage prefix scans and then filter in the web route. With large development databases this makes delegated tabs and run traces slow even when the client asks for only a small page.

## What Changes

- Keep canonical EventLog rows keyed by `events/{session_id}/{seq}`.
- Add secondary EventLog indexes for `session_id + source`, `session_id + agent_name`, and `session_id + event_type`.
- Add an EventLog query facade so web routes request scoped reads instead of replaying and filtering large vectors.
- Optimize Redb prefix listing so prefix reads use ordered range iteration rather than scanning the whole table.
- Clear existing development `sessions.db`; no backfill or historical migration is required in this slice.

## Impact

- Affected specs: `session-event-log`
- Affected code: `macaca-persist` EventLog/storage, `macaca-web` session events/run-trace routes, local development session DB
- Compatibility impact: existing append/replay callers keep compiling; old local development sessions are intentionally deleted before validation
