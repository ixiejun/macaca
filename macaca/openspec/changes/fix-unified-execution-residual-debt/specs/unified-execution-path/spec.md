## MODIFIED Requirements

### Requirement: All Application Types Converge To One Path

Macaca OS SHALL execute YAML, WASM, GenUI, and headless applications through the same Application ABI and the same canonical service path. Application type SHALL NOT select a separate execution backend. Application-owned UI surfaces that start, control, replay, or inspect application execution SHALL use the application execution bridge/service projection and SHALL NOT run an alternate LLM/tool loop for production execution.

#### Scenario: YAML and WASM produce one execution chain
- **WHEN** a YAML application and a WASM application each run one agent execution
- **THEN** service-call audit replay by session id SHALL show exactly one execution chain per run through the canonical path
- **AND** both SHALL reuse the same trace/audit correlation and replay path

#### Scenario: Application UI does not start a parallel execution engine
- **WHEN** an application-owned UI starts or replays a task
- **THEN** it SHALL call the application execution bridge/service projection
- **AND** it SHALL NOT start a separate browser-local LLM/tool execution loop as a production path

### Requirement: Removal Of Multi-Path Path-Selection Markers

Once execution paths are unified, Macaca OS SHALL remove the multi-path path-selection markers (`graph_owner`/`execution.graph_owner` discrimination, `authoritative`/`non_authoritative`/old-unmarked classification, `suppress_executor_lifecycle`, retired chat pause switches). Terminal-state determination SHALL treat all host commands as equally authoritative. Application-scoped agent selection SHALL use runtime-bound agent ids as the authoritative scope and SHALL NOT fall back to name-only selection when runtime bindings are missing.

#### Scenario: No reconciliation markers remain in production
- **WHEN** the codebase is scanned after path convergence
- **THEN** production code SHALL contain zero occurrences of old unmarked-path, non-authoritative-path, executor-lifecycle-suppression, and retired chat-pause switch tokens
- **AND** terminal completion/failure SHALL be computed without multi-path branching

#### Scenario: Markers are removed only after replay proves a single chain
- **WHEN** a reconciliation marker is proposed for removal
- **THEN** removal SHALL proceed only if audit replay first proves the related capability resolves through a single canonical chain
- **AND** the change SHALL keep deterministic terminal-state semantics

#### Scenario: Runtime app scope requires runtime-bound ids
- **WHEN** a shell or service adapter lists agents for one runtime application
- **THEN** selected manifests SHALL match both the declared application agent name and a runtime-bound agent id
- **AND** missing runtime bindings SHALL return no selected manifest rather than selecting by name alone
