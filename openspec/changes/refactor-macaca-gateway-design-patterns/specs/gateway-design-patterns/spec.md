## ADDED Requirements

### Requirement: Platform Neutral Gateway Messages

Gateway MUST expose platform-neutral inbound and outbound message primitives independent of Telegram or Discord wire formats.

#### Scenario: Telegram text maps to neutral inbound message

- **WHEN** a Telegram text message is parsed
- **THEN** the parser produces a platform-neutral inbound message with platform, user id, channel id, and content

#### Scenario: Neutral inbound message converts to existing event

- **WHEN** a neutral inbound task request is dispatched through the compatibility path
- **THEN** it produces the equivalent existing `GatewayEvent::TaskRequest`

### Requirement: Transport Boundary

Gateway MUST expose a transport trait that separates platform lifecycle and message sending from gateway orchestration.

#### Scenario: Existing adapters remain callable

- **WHEN** existing code constructs `TelegramAdapter` or `DiscordAdapter`
- **THEN** it can still start, send, and stop through the legacy adapter interface

#### Scenario: Transport sends reply

- **WHEN** a `GatewayTransport` receives a `GatewayReply`
- **THEN** it sends the reply through the platform-specific send implementation

### Requirement: Gateway Mediator Boundary

Gateway MUST provide a mediator boundary for message handling without depending on web, kernel, or application crates.

#### Scenario: Mediator handles task request

- **WHEN** the mediator receives a task request
- **THEN** it dispatches the equivalent existing `GatewayEvent` to an event sink

### Requirement: Reply Formatting Strategy

Gateway MUST format outgoing replies through platform-specific strategy objects.

#### Scenario: Telegram reply splitting remains stable

- **WHEN** a Telegram reply exceeds Telegram's max message length
- **THEN** it is split with the same newline-preferred behavior as the current implementation

#### Scenario: Plain text formatter leaves short replies intact

- **WHEN** a short plain text reply is formatted
- **THEN** it returns one unchanged message chunk

### Requirement: Config Driven Gateway Builder

Gateway MUST provide a builder or factory that constructs configured gateway adapters without caller-side platform branching.

#### Scenario: Disabled adapters are not registered

- **WHEN** gateway config disables Telegram or Discord
- **THEN** the builder does not register that adapter

#### Scenario: Enabled adapters are registered

- **WHEN** gateway config enables Telegram and Discord
- **THEN** the builder registers both configured adapters

### Requirement: Legacy Gateway Interfaces Are Deprecated But Retained

Gateway MUST mark replaced legacy interfaces deprecated without deleting them, so migration work can locate remaining callers.

#### Scenario: Legacy adapter remains available

- **WHEN** legacy code imports `ImAdapter`, `EventHandler`, or `Gateway`
- **THEN** the code can still compile with an explicit local deprecation allowance

#### Scenario: Production CLI uses builder path

- **WHEN** the CLI starts configured gateway adapters
- **THEN** it uses the config-driven builder instead of directly calling deprecated gateway lifecycle APIs
