## ADDED Requirements

### Requirement: Domain-Pack Contracts Live Outside Foundation Proto

The system SHALL NOT keep concrete domain-pack contract types (finance, commerce,
developer, ai, office, identity, media, etc.) in the foundation `macaca-proto`
crate. Such types SHALL live in a dedicated contracts crate outside the kernel
dependency closure. `macaca-proto` SHALL retain only provider-neutral domain-pack
framework types (traits, aggregator, generic DTO/command/error) and a data-driven,
self-registering pack registry rather than a name-branching match.

#### Scenario: Proto holds no concrete domain-pack semantics
- **WHEN** `macaca-proto` source is inspected
- **THEN** it SHALL NOT contain approval-classification, accounting-report,
  bounds, or preflight business rules for a concrete domain
- **AND** the kernel dependency closure SHALL NOT include concrete domain-pack
  contracts

#### Scenario: Pack registry is data-driven
- **WHEN** a domain and pack slug are resolved
- **THEN** resolution SHALL use a self-registration registry rather than a
  hardcoded `(domain, slug)` match arm

### Requirement: Payment Contracts And State Live In The Payment Service

The system SHALL NOT keep payment domain contracts (quotes, receipts, intents,
settlement state transitions) in the foundation `macaca-persist` crate. Foundation
persistence SHALL expose only neutral key-value and memento primitives.

#### Scenario: Foundation persistence carries no payment semantics
- **WHEN** `macaca-persist` source is inspected
- **THEN** it SHALL NOT define payment quote/receipt/intent/settlement types

### Requirement: Optional-Module Types Are Feature-Gated Out Of Foundation

The system SHALL feature-gate optional-module concerns (Web3 bridge types,
concrete transport providers such as NATS) so they are not compiled into the base
foundation / kernel dependency closure when disabled.

#### Scenario: Web3 types absent from base build
- **WHEN** the base OS is built without the web3 feature
- **THEN** foundation IPC SHALL NOT compile or export Web3 bridge types

#### Scenario: Concrete transport is optional
- **WHEN** the base OS is built without the NATS feature
- **THEN** the concrete NATS transport dependency SHALL NOT be linked

### Requirement: Foundation Default Config Is Provider-Neutral

The system SHALL NOT hardcode concrete provider, model, vendor URL, or gateway
names in foundation configuration defaults. Concrete provider values SHALL live in
application-layer configuration files.

#### Scenario: Default config names no vendor
- **WHEN** the foundation default configuration is constructed
- **THEN** it SHALL contain neutral/empty values, not a specific vector backend
  URL, embedding model name, vendor API base URL, or gateway provider name

### Requirement: Event Log Durability Is Truthful

The system SHALL propagate write failures from foundation event-log append
operations that claim durability rather than discarding them, and SHALL NOT
advance the sequence counter in a way that leaves gaps for failed writes.

#### Scenario: Failed append is reported
- **WHEN** an event-log append write fails
- **THEN** the failure SHALL be propagated (or the sequence rolled back) and
  recorded, not silently swallowed while returning a valid sequence number
