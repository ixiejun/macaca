## ADDED Requirements

### Requirement: OS-layer file-size governance SHALL include advisory headroom

Macaca SHALL keep the hard 500-line OS-layer production Rust source gate and SHALL add an advisory 450-line headroom diagnostic so maintainers split modules before they reach the constitutional limit.

#### Scenario: Production file exceeds advisory threshold
- **WHEN** an OS-layer production Rust source file is at or above 450 lines and at or below the hard limit
- **THEN** the advisory gate SHALL report the file path, line count, owning layer, and split guidance
- **AND** the advisory diagnostic SHALL NOT fail the build unless the file exceeds the hard limit or a future OpenSpec change promotes the rule

#### Scenario: Production file exceeds hard threshold
- **WHEN** an OS-layer production Rust source file exceeds 500 lines
- **THEN** the hard file-size gate SHALL fail
- **AND** the diagnostic SHALL instruct maintainers to split by ownership rather than formatting

### Requirement: Repeated scan hotspots SHALL have deterministic audit guidance

Macaca SHALL identify repeated linear-scan hotspots in request/event/task-board paths and SHALL prefer local indexes that preserve ordering, scope filtering, authorization behavior, missing-record behavior, and existing response shapes.

#### Scenario: Hot path uses repeated membership scans
- **WHEN** a request/event/task-board path repeatedly scans manifests, tasks, aliases, skills, descriptors, or memory rows for membership or lookup
- **THEN** the implementation SHALL either build a local `HashSet`/`HashMap` index or document why the path is cold/small-N
- **AND** tests SHALL preserve ordering and missing-record behavior when indexing is introduced

### Requirement: Protocol DTO modules SHALL be split by command family

Macaca protocol modules SHALL avoid becoming shared semantic dumping grounds. Dense `macaca-proto` modules SHALL be split by command family while preserving public type names, serde compatibility, and re-export stability.

#### Scenario: Dense protocol module is split
- **WHEN** a dense protocol DTO module is split
- **THEN** command/result/state types SHALL remain provider-neutral
- **AND** public re-exports SHALL preserve existing caller compatibility unless a later breaking-change proposal says otherwise
- **AND** serde roundtrip tests SHALL prove compatibility
- **AND** business state transitions SHALL remain in owning services or kernel primitives, not in protocol DTO modules
