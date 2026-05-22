## Context

Heartbeat is a native autonomy service, not a Scheduler job. Applications declare
which agents may participate in heartbeat execution, but Heartbeat owns cadence,
gates, profiles, wake mementos, and audit evidence.

## Goals / Non-Goals

- Goal: each declared heartbeat agent receives a distinct Heartbeat profile.
- Goal: each profile can carry independent fixed interval and cooldown policy.
- Goal: all calls remain provider-neutral, traced, audited, and shell-safe.
- Non-goal: raw manifest editing through Web/frontend.
- Non-goal: Scheduler-owned heartbeat timing.

## Decisions

- Use runtime-host as the Adapter from sanitized Application Service declaration
  views to Heartbeat native profile registration.
- Keep manifest `profile_id` as a selector and add concrete `native_profile_id`
  for the Heartbeat profile that operators edit.
- Use `application:{app_id}.agent:{agent_name}.heartbeat` as the scope key. The
  agent name is a manifest-owned identifier, not an application-specific branch.
- Add explicit fixed interval and cooldown fields to profile summaries and
  update commands so the UI does not hide policy in arbitrary metadata.
- Filter dispatch by accepted profile id/scope key. Legacy app-scoped wakes keep
  the previous all-declarations behavior for migration compatibility.

## Risks / Trade-offs

- DTO expansion has broad impact. Mitigation: additive fields, focused tests, and
  GitNexus review.
- Per-profile cooldown changes gate behavior. Mitigation: missing cooldown keeps
  the existing provider default.
- Multiple profiles increase run count. Mitigation: Web routes query bounded
  histories and aggregate only app-owned heartbeat scope keys.
