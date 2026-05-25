## ADDED Requirements

### Requirement: Evolution benchmarks SHALL compare normalized paired metrics

The Autonomy Evolution Control Plane SHALL expose a provider-neutral paired
benchmark command that compares baseline and candidate measurements for the same
task family using a standard metric schema.

#### Scenario: Quality-preserving efficiency gain passes
- **GIVEN** baseline and candidate measurements share the same task family id
- **AND** both measurements include bounded evidence refs and required metric
  fields
- **AND** candidate quality is preserved
- **AND** candidate efficiency improves without material regression
- **WHEN** the paired benchmark command is evaluated
- **THEN** the benchmark decision SHALL be `Passed`
- **AND** the result SHALL include bounded metric deltas and reason codes

#### Scenario: Quality regression fails even with efficiency gain
- **GIVEN** candidate tokens, elapsed time, or tool calls improve
- **AND** candidate quality score regresses beyond the configured tolerance
- **WHEN** the paired benchmark command is evaluated
- **THEN** the benchmark decision SHALL be `Failed`
- **AND** the failure reason SHALL identify quality regression

#### Scenario: Non-comparable task families are inconclusive
- **GIVEN** baseline and candidate measurements use different task family ids
- **WHEN** the paired benchmark command is evaluated
- **THEN** the benchmark decision SHALL be `Inconclusive`
- **AND** the result SHALL NOT claim an optimization pass

#### Scenario: Missing required metric evidence is inconclusive
- **GIVEN** either measurement lacks required metrics or bounded evidence refs
- **WHEN** the paired benchmark command is evaluated
- **THEN** the benchmark decision SHALL be `Inconclusive`
- **AND** the missing fields SHALL be represented as bounded reason codes

#### Scenario: Regression reasons fail benchmark
- **GIVEN** a candidate measurement carries regression reasons
- **WHEN** the paired benchmark command is evaluated
- **THEN** the benchmark decision SHALL be `Failed`
- **AND** the result SHALL preserve sanitized regression reason codes
