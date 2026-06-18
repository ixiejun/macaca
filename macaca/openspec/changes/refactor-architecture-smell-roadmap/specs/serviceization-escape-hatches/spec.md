## ADDED Requirements

### Requirement: Boundary gate tests SHALL be split by policy category

Macaca SHALL keep boundary tests executable and maintainable by splitting oversized policy bundles into policy-specific gate modules with shared fixtures.

#### Scenario: Oversized boundary test is refactored
- **WHEN** a boundary gate file combines unrelated shell, SDK, kernel, provider, trace, audit, and serviceization assertions
- **THEN** it SHALL be split into smaller policy-category modules
- **AND** shared fixtures SHALL live in support modules
- **AND** each resulting gate SHALL retain deterministic diagnostics and equivalent assertion coverage

### Requirement: Static process-local state SHALL declare lifecycle ownership

Macaca production code SHALL NOT add hidden process-local state without English comments documenting owner, lifecycle, initialization, reset/test isolation, restart semantics, and why explicit composition-root state is not used.

#### Scenario: Static registry or lock exists
- **WHEN** production code defines or keeps a `OnceLock`, static mutex, static registry, or process-local singleton
- **THEN** the module SHALL explain the owner boundary, lifecycle, reset/test-isolation behavior, restart behavior, and audit implications
- **AND** callers SHALL still receive structured unavailable or Null Object behavior when the underlying provider is absent

#### Scenario: New hidden static state lacks lifecycle documentation
- **WHEN** a production change adds hidden process-local state without lifecycle documentation
- **THEN** the escape-hatch gate SHALL fail with file, line, token, and replacement guidance

### Requirement: Text and name routing SHALL move to typed descriptors or declarative mappings

Macaca OS-layer routing SHALL not depend on hardcoded business, provider, application, model, gateway, driver, payment, chain, workflow, planner, worker, or review names. Remaining text matching SHALL be limited to ingestion boundaries and SHALL route through typed capability descriptors, declarative mapping records, or audited fallback policies.

#### Scenario: OS-layer routing branches on hardcoded names
- **WHEN** production OS-layer source branches on hardcoded application, provider, model, driver, gateway, chain, payment, workflow, planner, worker, or review names
- **THEN** the no-hardcoded-name or escape-hatch gate SHALL fail
- **AND** the diagnostic SHALL identify the descriptor, manifest, policy, or mapping source that should provide the value

#### Scenario: Ingestion boundary performs declarative matching
- **WHEN** text matching is unavoidable at an ingestion boundary
- **THEN** matching patterns SHALL come from declarative mappings, descriptors, manifests, or policy-owned data
- **AND** the code SHALL emit sanitized reason codes for accepted, rejected, and fallback decisions
