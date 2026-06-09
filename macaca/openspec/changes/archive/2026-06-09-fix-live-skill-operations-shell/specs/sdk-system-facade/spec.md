## MODIFIED Requirements

### Requirement: Unsupported service operations SHALL fail structurally

SDK system clients and shell adapters SHALL return structured unavailable, configuration, or unsupported errors for operations whose concrete service providers are absent or whose live runtime target is not configured.

#### Scenario: Service call has no backing service
- **WHEN** a service call command targets a capability that has no S3 backing client
- **THEN** the SDK client SHALL return a structured unavailable or unsupported error
- **AND** it SHALL NOT panic, hang, silently succeed, or construct a concrete provider

#### Scenario: CLI live Skill target is unavailable
- **WHEN** a CLI Skill command is app-scoped to a local live API base that is not reachable
- **THEN** CLI SHALL return a structured configuration or unavailable error
- **AND** it SHALL NOT fake an empty governance snapshot or mutate local placeholder state
