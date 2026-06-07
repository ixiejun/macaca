## ADDED Requirements

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
