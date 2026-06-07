# serviceization-escape-hatches Specification

## ADDED Requirements

### Requirement: Reject Autonomy Loops Outside Runtime Host

Serviceization gates SHALL reject new production autonomy background loops,
cron tickers, heartbeat tickers, due-run dispatchers, or recovery wake loops
outside approved runtime-host autonomy supervisor surfaces.

#### Scenario: Shell adds cron ticker

Given a change adds a cron ticker, heartbeat ticker, due-run dispatcher, or
recovery wake loop inside Web, CLI, frontend, SDK, kernel, or application code
When serviceization escape-hatch gates evaluate the change
Then the gates fail with guidance to move the loop into runtime-host autonomy
supervisor wiring.

### Requirement: Reject Local Autonomy Provider Construction Outside Runtime Host

Serviceization gates SHALL reject construction of local Scheduler providers,
local Heartbeat providers, or autonomy supervisors outside approved runtime-host
composition roots.

#### Scenario: SDK constructs local scheduler provider

Given SDK or SystemFacade code constructs a local Scheduler provider
When dependency and escape-hatch gates evaluate the change
Then the gates fail because SDK may only build provider-neutral client commands
and provider construction belongs to runtime-host.

### Requirement: Reject Application-Specific Autonomy Activation

Serviceization gates SHALL reject autonomy activation, supervisor dispatch,
heartbeat recovery, and scheduler target logic that branches on application,
workflow, provider, driver, model, gateway, chain, payment, or business-domain
names.

#### Scenario: Supervisor branches on workflow name

Given the autonomy supervisor adds special handling for a workflow or
application name
When serviceization gates evaluate the change
Then the gates fail because local autonomy runtime must dispatch only generic
provider-neutral target categories.
