## ADDED Requirements

### Requirement: Absent Or Unimplemented Providers Return Structured States

The system SHALL return a structured `unavailable` / `unsupported` / `denied`
result when a provider-backed capability is absent, unconfigured, or
unimplemented. It SHALL NOT report fake success, silently drop the operation,
return an empty success payload indistinguishable from a real empty result,
crash, or hang.

#### Scenario: Missing gateway credential is structural, not silent success
- **WHEN** a gateway adapter has no configured token and a send is attempted
- **THEN** the call SHALL return a structured unavailable result rather than an
  `Ok` that discards the message

#### Scenario: Stub adapter reports unsupported
- **WHEN** an adapter is a non-functional stub but is registered as enabled
- **THEN** its operations SHALL return a structured unsupported result, or
  registration SHALL refuse to enable it

#### Scenario: Empty result is distinguishable from unavailability
- **WHEN** an orchestration query runs with no provider wired
- **THEN** it SHALL return a structured unavailable state distinct from a
  legitimately empty (zero-item) success

### Requirement: Partial Setup Failures Roll Back Instead Of Leaving Zombies

Multi-step create/registration flows SHALL roll back earlier steps when a later
step fails, so no half-created record remains active and observable.

#### Scenario: Registration failure rolls back prepared state
- **WHEN** a task/job is prepared and persisted but its downstream registration
  returns not-accepted or errors
- **THEN** the prepared state and any stored payload SHALL be removed so no
  permanently active zombie record remains

### Requirement: Health Checks Reflect Real Dependency Availability

Service health SHALL reflect real underlying availability rather than a hardcoded
healthy constant, including the availability of dependencies the service requires.

#### Scenario: Unavailable dependency degrades health
- **WHEN** a service depends on another provider that is unavailable
- **THEN** its health SHALL report degraded/unavailable rather than healthy
