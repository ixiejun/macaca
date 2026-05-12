## ADDED Requirements

### Requirement: Sanitized Application Metadata Query
Application Service SHALL provide traced metadata query commands that return sanitized application, ability, entry, policy, overlay, and manifest digest views.

#### Scenario: Shell queries metadata
- **WHEN** Web, CLI, Gateway, or framework adapters need application entry, agent list, ability list, tool policy, context policy, skill policy, or MCP overlay metadata
- **THEN** they SHALL query Application Service through `SystemApplicationClient` instead of interpreting raw manifests in new production code.

#### Scenario: Query is traced
- **WHEN** a metadata query is created
- **THEN** it SHALL carry trace context and application/session scope where applicable, or return fail-closed before provider dispatch.

### Requirement: Sanitized Metadata Views
Metadata views SHALL expose only bounded, safe information required by shells and framework adapters.

#### Scenario: Unsafe fields are excluded
- **WHEN** a metadata view is serialized, logged, or returned to Web/CLI
- **THEN** it SHALL NOT include prompt bodies, raw full manifest bodies, raw agent configs, env values, API keys, secrets, private keys, raw host payloads, or unbounded user input.

#### Scenario: Safe fields are available
- **WHEN** a shell needs application metadata
- **THEN** views MAY include ids, names, versions, runtime kind, ability kind, entry references, declared capability names, policy flags, counts, digests, safe path metadata, status, and structured diagnostics.

### Requirement: Web Uses Metadata Service First
The Web shell SHALL prefer Application Service sanitized metadata views for application-owned metadata and retain deprecated raw manifest fallback only for compatibility.

#### Scenario: Chat preflight uses metadata view
- **WHEN** `/api/chat/v2` performs application preflight
- **THEN** it SHALL prefer sanitized Application Service metadata for entry-agent and session envelope decisions while preserving existing coordinator execution behavior.

#### Scenario: Deprecated fallback remains bounded
- **WHEN** the service-backed metadata path is unavailable during migration
- **THEN** Web MAY use deprecated raw manifest fallback, but the fallback SHALL be marked as compatibility debt and SHALL NOT be expanded for new production behavior.

### Requirement: Application Service Does Not Execute External Capabilities
The sanitized metadata service SHALL NOT execute Task, LLM, Memory, Context, Driver, Skill, MCP, Plugin, Payment, Web3, EVM, or business workflow behavior.

#### Scenario: Metadata query avoids provider execution
- **WHEN** a metadata query requests tool, skill, MCP, context, or capability declarations
- **THEN** Application Service SHALL return declarations/projections only and SHALL NOT invoke the corresponding provider or service execution path.
