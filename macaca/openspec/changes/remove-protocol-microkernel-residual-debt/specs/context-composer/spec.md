## ADDED Requirements

### Requirement: Context Composition SHALL Have No Deprecated Entry Points

Context composition SHALL expose canonical provider, composer, plan, report, and facade APIs only. Default behavior SHALL be modeled as a named current strategy, not as an old-entry fallback path.

#### Scenario: Runtime assembles context through canonical facade
- **WHEN** runtime or framework code needs model-visible context
- **THEN** it SHALL call the canonical context facade/composer API
- **AND** it SHALL NOT call old prompt builders, old engine constructors, or deprecated context wrappers

#### Scenario: Default strategy is unavailable
- **WHEN** a requested context provider or composer strategy is unavailable
- **THEN** the context service SHALL return structured unavailable/unsupported or use a policy-approved default strategy
- **AND** it SHALL emit a sanitized context report without old-path terminology

## REMOVED Requirements

### Requirement: Legacy context entry points SHALL remain deprecated and searchable

**Reason**: The terminal cleanup forbids retained deprecated entry points and searchable old wrappers.

**Migration**: Replace old context entry points with canonical context composer/facade APIs and service-level tests.
