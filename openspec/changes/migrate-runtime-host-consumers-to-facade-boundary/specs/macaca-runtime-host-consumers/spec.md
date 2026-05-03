## ADDED Requirements

### Requirement: Upper Consumers Use Runtime-Host Boundary

Repository upper consumers SHALL depend on `macaca-runtime-host` stable consumer-facing exports instead of local compatibility thin shells.

#### Scenario: Web route probes MCP status through runtime-host import

- **WHEN** `macaca-web` exposes the MCP status route
- **THEN** it imports MCP status types and policy types directly from `macaca-runtime-host`
- **AND** it does not depend on a web-local `mcp_runtime` thin shell

### Requirement: Web Toolkit And Skill MCP Consumers Avoid Legacy Thin Shell

`macaca-web` toolkit assembly and skill-backed MCP probing SHALL import runtime-host consumer primitives directly rather than through `crate::mcp_runtime::*`.

#### Scenario: Toolkit registration imports runtime-host directly

- **WHEN** `build_toolkit` assembles MCP definitions and registers MCP tools
- **THEN** it uses `macaca-runtime-host` consumer-facing exports
- **AND** MCP registration behavior remains compatible with current runtime behavior

#### Scenario: Skill MCP probe imports runtime-host directly

- **WHEN** `probe_skill_mcp_servers` resolves definitions and probes MCP runtime state
- **THEN** it uses `macaca-runtime-host` consumer-facing exports
- **AND** the returned `SkillMcpStatus` mapping remains unchanged

### Requirement: Deprecated Runtime-Host Paths Stay Internal

Repository upper consumers SHALL NOT call runtime-host deprecated compatibility APIs or equivalent web-local re-export paths.

#### Scenario: Deprecated usage scan

- **WHEN** repository upper consumers are scanned for deprecated runtime-host manager APIs and `crate::mcp_runtime::*` imports
- **THEN** no `macaca-web` executable call sites remain on those paths
- **AND** deprecated compatibility APIs remain available only inside `macaca-runtime-host` itself

### Requirement: Web Thin Shell Removal

`macaca-web` SHALL remove its local `mcp_runtime` thin shell once all in-crate consumers have migrated.

#### Scenario: Thin shell no longer exists

- **WHEN** all `macaca-web` MCP consumer modules are migrated
- **THEN** `macaca-web/src/mcp_runtime.rs` is removed
- **AND** `macaca-web/src/lib.rs` no longer declares `pub mod mcp_runtime;`
