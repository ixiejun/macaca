## ADDED Requirements

### Requirement: Data Retrieval Result Events Reach Session EventLog

The Web runtime adapter SHALL mirror session-scoped generic data retrieval result evidence into the session EventLog using bounded audit-safe fields from host-command results.

#### Scenario: Data host command returns during a session

- **WHEN** a generic host dispatch returns `service.call` host-command result evidence for a session
- **THEN** the session EventLog SHALL receive `service_call_audit` events for each result
- **AND** live Web UI subscribers SHALL receive matching SSE events after persistence
- **AND** the payload SHALL include only audit-safe fields such as stage, result index, status, trace id, service id, operation, provider id, and output hash
- **AND** the payload SHALL NOT include raw provider output, raw input, credentials, prompts, manifests, package bytes, or application-specific semantics

### Requirement: Data Retrieval Visibility Does Not Change Policy

Session-visible data retrieval result events SHALL NOT change service routing, allowlist, policy, resource, entitlement, or provider selection behavior.

#### Scenario: Service call is denied

- **WHEN** service policy denies a call
- **THEN** the visibility adapter SHALL NOT retry, override, or bypass the policy decision
- **AND** it SHALL NOT retry, override, or bypass the policy decision
