# Autonomy Application Operations Placement Design

## Context

The current Autonomy schedule management panel is reachable from the chat workspace tab area. That placement is misleading because the scheduler and heartbeat capabilities are application-level operating-system services. They are not owned by a single session, a coordinator trace, or a delegated agent execution.

Macaca OS is designed as a provider-neutral, serviceized Agent OS. Application-facing operating capabilities must remain generic, auditable, and reusable across all applications. The UI should communicate those boundaries as clearly as the backend contracts do.

## Decision

Move Autonomy out of the session/agent workspace tab group and expose it through an application-level Operations dialog opened from a generic application action button.

The Operations dialog is a generic application management surface. It can host Autonomy now and later host other application-scoped OS capabilities such as heartbeat status, runtime health, audit evidence, entitlement state, or service availability. It must not contain application-specific workflows, app names, provider names, driver names, or business logic.

## Design Pattern Fit

This design follows the Facade and Composite patterns:

- The Operations dialog acts as a UI facade over application-scoped system capabilities.
- Each capability panel, starting with schedule management, remains an independent component with its own data facade.
- The chat/session workspace remains focused on conversation, session traces, and agent execution state.

This avoids a God Object page while keeping the first implementation small and reversible.

## Layout Model

The chat page will keep three conceptual regions:

1. Application/session navigation on the left.
2. Session workspace in the center, including conversation, session surfaces, and agent/session tabs.
3. Existing session-adjacent status panels such as agents and task health on the right.

Autonomy will appear in a modal dialog opened from an application-level button in the page header. The existing `ScheduleManagerPanel` can be reused, but its owner should become the application operations dialog instead of the workspace tab switcher or the persistent right rail.

## Data Flow

The data flow remains unchanged:

1. The frontend Autonomy facade calls `/api/apps/{appId}/autonomy/...`.
2. The web shell routes through serviceized Scheduler contracts.
3. The Scheduler provider enforces application scoping and emits traceable/auditable mutation results.

Only presentation ownership changes. No scheduler API, provider contract, or application-specific behavior is introduced.

## Error Handling

The Operations dialog should surface generic capability-level errors:

- unavailable scheduler provider
- invalid application identifier
- failed schedule mutation
- failed run history load

Errors must remain generic and tied to service capability status, not to any application-specific workflow.

## Logging and Traceability

The existing backend route and Scheduler provider logging remain the authoritative audit path. The UI placement change should preserve trace identifiers returned by the schedule APIs and keep them visible where the panel already renders them.

If future Operations panels are added, each panel should expose its own service trace or audit handle instead of sharing session trace state.

## Scope

In scope:

- Remove Autonomy from the session/agent workspace tab list.
- Add a generic application Operations button on the chat page.
- Mount `ScheduleManagerPanel` inside an application-level Operations dialog.
- Preserve the existing visual language.
- Preserve existing frontend Autonomy data facade and backend serviceized routes.

Out of scope:

- Creating a new dedicated `/apps/{appId}/autonomy` route.
- Replacing the existing right-side agents/task/status panel with Autonomy.
- Adding application-specific schedule templates or workflow presets.
- Changing Scheduler provider semantics.
- Adding heartbeat management UI in this slice.
- Changing the left application/session navigation model.

## Validation

Validation should prove:

- The Autonomy panel is no longer rendered under the session/agent workspace tab group or the persistent right rail.
- The panel still loads schedules from `/api/apps/{appId}/autonomy/schedules`.
- Existing schedule CRUD behavior still works.
- Frontend lint passes.
- Existing backend route/serviceization tests remain green if backend code is touched.

## Risks

- The chat page is already large, so the implementation should avoid adding more responsibilities directly into the page file. If new UI structure is needed, extract small focused components.
- A right-side rail may compete with existing trace panels. The first version should use collapsible or clearly bounded layout behavior if space is tight.
- Because this is a presentation ownership change, tests should focus on DOM placement and API continuity rather than backend behavior.
