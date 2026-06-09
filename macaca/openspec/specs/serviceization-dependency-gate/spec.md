# serviceization-dependency-gate Specification

## Purpose
Executable Route C workspace dependency boundary enforcement for Macaca Agent OS: allowlist governance, forbidden-edge classification, and terminal zero-row migration proof. Baseline aligned to P5 microkernel refactor by `refactor-unified-call-path-microkernel`.
## Requirements
### Requirement: Macaca SHALL provide an executable Route C dependency boundary gate

Macaca SHALL provide an executable dependency boundary gate that inspects direct workspace crate dependencies and enforces Route C microkernel/serviceization boundaries.

#### Scenario: Dependency gate evaluates workspace metadata

- **WHEN** the dependency boundary test runs
- **THEN** it SHALL execute or consume `cargo metadata --no-deps --format-version 1`
- **AND** it SHALL inspect direct dependency edges between workspace crates
- **AND** it SHALL produce deterministic results without requiring network access, a frontend server, a real LLM provider, a browser, Web3, EVM, or external services

### Requirement: Macaca SHALL classify every workspace crate into a Route C layer

Macaca SHALL classify every workspace crate into a stable architecture layer before evaluating dependency rules.

#### Scenario: Unknown workspace crate fails classification

- **WHEN** a workspace crate is missing from the dependency gate layer map
- **THEN** the gate SHALL fail with an actionable diagnostic naming the crate
- **AND** the diagnostic SHALL instruct maintainers to classify the crate through OpenSpec before relying on it

#### Scenario: Known crates map to microkernel boundary layers

- **WHEN** the gate classifies current workspace crates
- **THEN** classifications SHALL align with `macaca/docs/agent-os-microkernel-boundaries.md`
- **AND** kernel, service provider, runtime host, application framework, presentation shell, optional module, IPC/service bus, proto, and integration-test layers SHALL be distinguishable

### Requirement: Macaca SHALL enforce initial forbidden dependency specifications

Macaca SHALL enforce initial forbidden dependency rules for kernel/provider coupling, presentation/provider coupling, CLI/Web-internal coupling, optional-module base dependency leakage, and provider/presentation reverse coupling.

#### Scenario: New kernel provider dependency is rejected

- **WHEN** `macaca-kernel` adds a new direct dependency on a provider implementation crate outside the allowlist
- **THEN** the dependency boundary gate SHALL fail
- **AND** the diagnostic SHALL include rule id `kernel-no-provider-deps`, the source crate, the target crate, rationale, and replacement service path

#### Scenario: New presentation provider construction dependency is rejected

- **WHEN** a presentation shell crate adds a new direct dependency on a provider implementation crate outside the allowlist
- **THEN** the gate SHALL fail
- **AND** the diagnostic SHALL include rule id `presentation-no-provider-construction-hub`

#### Scenario: Service provider depends on presentation shell

- **WHEN** a service provider crate depends on a presentation shell crate
- **THEN** the gate SHALL fail
- **AND** the diagnostic SHALL include rule id `service-provider-no-presentation`

### Requirement: Macaca SHALL represent current violations through a migration allowlist

Macaca SHALL represent current dependency boundary violations as explicit migration allowlist entries rather than treating them as acceptable architecture.

#### Scenario: Allowlisted current violation passes with migration metadata

- **WHEN** a current direct dependency edge violates a boundary rule but appears in the allowlist
- **THEN** the gate MAY pass that edge
- **AND** the allowlist row SHALL include rule id, from crate, to crate, current reason, replacement service/facade path, target migration phase, expiry condition, and owner/status

#### Scenario: New exception requires OpenSpec update

- **WHEN** a developer needs to add a new exception to the dependency boundary gate
- **THEN** they SHALL update the OpenSpec change or baseline specification and `macaca/docs/route-c-serviceization-allowlist.md`
- **AND** the exception SHALL NOT be added silently in code only

### Requirement: Macaca SHALL keep S0 additive and non-migrating

S0 SHALL add dependency gate infrastructure and documentation without removing provider dependencies or changing runtime behavior.

#### Scenario: Existing user-visible flows remain unchanged

- **WHEN** S0 is implemented
- **THEN** YAML application loading, `/api/chat/v2`, trace, task board, resume, driver, skill/MCP, Web UI, and CLI behavior SHALL continue through existing paths
- **AND** S0 SHALL NOT implement ServiceRuntime v1 or migrate Task, LLM, Memory, Driver, Skill, MCP, Gateway, Payment, Web3, or EVM providers

### Requirement: Macaca SHALL update architecture governance with dependency gate rules

Macaca SHALL update Route C architecture governance documentation to describe the executable dependency gate and allowlist process.

#### Scenario: Governance doc explains boundary enforcement

- **WHEN** maintainers read `macaca/docs/route-c-architecture-governance.md`
- **THEN** it SHALL state that dependency boundary violations must be represented as failing tests or documented allowlist rows
- **AND** it SHALL state that new provider dependencies in kernel or presentation shell crates require OpenSpec and allowlist updates

### Requirement: Macaca SHALL document new dependency gate code with detailed English comments

All new S0 Rust test/helper code SHALL include detailed English comments explaining dependency graph traversal, layer ownership, rule evaluation, allowlist semantics, diagnostics, and non-goals.

#### Scenario: Maintainer can audit the dependency gate from code and diagnostics

- **WHEN** a maintainer reads the dependency boundary test or observes a violation
- **THEN** comments and diagnostics SHALL explain what was checked, why the edge is forbidden, whether an allowlist entry exists, and which service/facade replacement path should be used
- **AND** diagnostics SHALL be deterministic and audit-friendly

### Requirement: Dependency Allowlist SHALL Reach Zero At Terminal State

The Route C dependency boundary gate SHALL enforce a terminal-state invariant that the migration allowlist contains zero rows. Any remaining allowlist row SHALL fail the gate, because at terminal state every forbidden edge has been removed rather than tolerated.

#### Scenario: Non-empty allowlist fails terminal gate
- **WHEN** the terminal dependency gate runs and the allowlist contains one or more rows
- **THEN** the gate SHALL fail and name each remaining row's rule id, source crate, target crate, and replacement boundary
- **AND** the diagnostic SHALL state that terminal state requires the edge to be removed, not allowlisted

#### Scenario: Zero allowlist passes terminal gate
- **WHEN** all forbidden direct dependency edges have been removed and the allowlist is empty
- **THEN** the gate SHALL pass
- **AND** `cargo metadata --no-deps` SHALL confirm none of the previously allowlisted edges exist

### Requirement: No Direct Provider Call Audit Gate

Macaca SHALL provide an executable audit gate proving that each serviceized capability is invoked only through its service client / facade. Direct provider invocation outside the canonical service path SHALL fail the gate.

#### Scenario: Direct capability provider call is rejected
- **WHEN** production code calls an LLM, tool, driver, skill, MCP, task, memory, context, payment, web3, or EVM provider directly instead of through the service client
- **THEN** the no-direct-provider-call gate SHALL fail with file, line, capability name, and the required service-client replacement
- **AND** the gate SHALL be deterministic and require no network, frontend, browser, or real provider

### Requirement: No Hardcoded Application Or Provider Names Gate

Macaca SHALL provide an executable audit gate proving that OS-layer production code contains no hardcoded agent/application/provider/model/driver/gateway/chain/payment names, with fixtures and tests excluded.

#### Scenario: Hardcoded role or provider name is rejected
- **WHEN** OS-layer production code hardcodes a name such as `coordinator`, `planner`, `worker`, an LLM provider name, a model name, a driver name, a gateway name, a chain name, or a payment name
- **THEN** the no-hardcoded-name gate SHALL fail with file, line, and the manifest/descriptor/config source that should supply the value

### Requirement: Shell Is Not A Semantic Owner Gate

Macaca SHALL provide an executable audit gate proving that presentation shells do not own system semantics: shells SHALL NOT drive task/loop execution directly, read deprecated direct provider fields, or contain agent-execution implementations.

#### Scenario: Shell semantic ownership is rejected
- **WHEN** a presentation shell drives kernel/executor task loops directly, reads deprecated `AppState` provider fields, or builds/executes agents in shell code
- **THEN** the shell-not-semantic-owner gate SHALL fail with file, line, and the facade/service replacement

### Requirement: OS-Layer File Size Gate

Macaca SHALL provide an executable audit gate proving that OS-layer Rust source files do not exceed 500 lines, treating oversized files as unclear ownership.

#### Scenario: Oversized OS source file is rejected
- **WHEN** an OS-layer Rust source file exceeds 500 lines
- **THEN** the file-size gate SHALL fail with the file path and line count
- **AND** the diagnostic SHALL instruct maintainers to split by ownership, not by formatting

### Requirement: Shell Dependency Purity Gate

Presentation shells SHALL converge on minimal workspace dependencies. `macaca-cli` SHALL depend only on `macaca-sdk` and `macaca-proto` among workspace crates. `macaca-web` SHALL converge to the same terminal set; until convergence completes, any extra workspace dependency SHALL be tracked by an executable shell dependency baseline gate that rejects new edges.

#### Scenario: CLI shell dependency tree is terminal-pure
- **WHEN** `cargo metadata --no-deps` evaluates `macaca-cli` workspace dependencies
- **THEN** the only workspace dependencies SHALL be `macaca-proto` and `macaca-sdk`

#### Scenario: Web shell extra dependencies are frozen not growing
- **WHEN** `macaca-web` still carries migration workspace dependencies beyond proto/sdk
- **THEN** the shell dependency purity gate SHALL permit only the documented baseline set
- **AND** any new workspace dependency outside the baseline SHALL fail the gate

