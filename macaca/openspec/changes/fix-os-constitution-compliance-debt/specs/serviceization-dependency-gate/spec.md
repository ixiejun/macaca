## ADDED Requirements

### Requirement: Provider Construction Gate Uses Naming Patterns And Mandatory Registration

The provider-construction gate SHALL detect provider construction by naming
pattern (e.g. `*ServiceProvider::`, `*Provider::new`) rather than a fixed token
deny-list, and SHALL require every new provider type to be registered in a
gate-known inventory. A newly introduced provider type that is not registered
SHALL fail the gate.

#### Scenario: Unregistered new provider fails the gate
- **WHEN** a new provider type is constructed outside the approved composition
  root and is not present in the gate inventory
- **THEN** the provider-construction gate SHALL fail

#### Scenario: Pattern match is not a fixed token list
- **WHEN** the gate evaluates source for provider construction
- **THEN** it SHALL match constructor naming patterns rather than an enumerated
  list of specific type tokens

### Requirement: Literal-Splitting Evasion Of Name Gates Is Prohibited

The gate SHALL treat as a violation any source that assembles a forbidden
provider/model/application name from split literals (e.g.
`concat!("claude","-code")`) specifically to evade the no-hardcoded-names gate.

#### Scenario: Split-literal name assembly is flagged
- **WHEN** OS-layer production source assembles a provider/application name via
  `concat!` or adjacent string fragments that reconstruct a gated name
- **THEN** the no-hardcoded-names gate SHALL flag it as a violation

### Requirement: Use-Level Boundary Scan Complements Dependency-Graph Gate

In addition to the Cargo-metadata dependency-graph gate, the boundary gates SHALL
include a `use`/path-level scan that detects illegal cross-layer usage which the
crate-dependency graph alone cannot observe.

#### Scenario: Illegal intra-allowed-dependency use is caught
- **WHEN** a crate uses an out-of-layer symbol reachable only because a broader
  dependency is allowed
- **THEN** the use-level scan SHALL flag the illegal usage

### Requirement: Contract Files Obey The OS-Layer File-Size Gate

Domain-pack and other OS-layer contract source files SHALL obey the 500-line hard
gate and 450-line advisory report, with no size exception rows.

#### Scenario: Oversized contract file fails the gate
- **WHEN** an OS-layer contract source file exceeds 500 production lines
- **THEN** the file-size gate SHALL fail until the file is split by stable
  ownership
