## ADDED Requirements

### Requirement: Production Gateway Consumers Use Pattern Primitives

Production upper crates that start gateway adapters SHALL use non-deprecated gateway primitives such as `GatewayBuilder` instead of directly calling deprecated gateway lifecycle APIs.

#### Scenario: CLI starts gateway from configuration
- **WHEN** gateway is enabled in CLI configuration
- **THEN** CLI starts it through `GatewayBuilder`
- **AND** CLI does not manually register Telegram or Discord adapters

### Requirement: Gateway Consumer Tests Cover New Primitives

Gateway integration tests SHALL cover the new builder, mediator, and transport primitives.

#### Scenario: Builder constructs enabled adapters
- **WHEN** integration tests build a gateway from enabled Telegram and Discord config
- **THEN** the resulting gateway contains both adapters

#### Scenario: Mediator dispatches inbound messages
- **WHEN** integration tests send a platform-neutral inbound message to `GatewayMediator`
- **THEN** the configured event sink receives the equivalent gateway event

#### Scenario: Transport sends platform-neutral replies
- **WHEN** integration tests send a `GatewayReply` through a configured transport
- **THEN** the transport accepts the platform-neutral reply without requiring the deprecated adapter API

### Requirement: Deprecated Gateway Calls Are Contained

Deprecated gateway APIs SHALL remain defined for compatibility, but direct calls from upper crates SHALL be migrated to gateway pattern primitives.

#### Scenario: Upper consumers avoid deprecated lifecycle APIs
- **WHEN** scanning `macaca-cli` and `macaca-integration-tests`
- **THEN** they do not call deprecated `Gateway`, `ImAdapter`, or `EventHandler` lifecycle APIs
- **AND** any remaining deprecated gateway API usage is contained within `macaca-gateway` compatibility code or tests
