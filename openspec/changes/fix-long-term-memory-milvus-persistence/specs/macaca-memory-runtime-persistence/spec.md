## ADDED Requirements

### Requirement: Web memory runtime SHALL use configured long-term backend

When local memory tools or active vector memory are enabled, the web composition root SHALL build the Memory runtime from `MacacaConfig.memory` rather than hardcoding an in-memory vector backend.

#### Scenario: Config selects Milvus

- **GIVEN** `memory.vector.backend` is `milvus`
- **AND** memory runtime exposure is enabled
- **WHEN** `macaca-web` builds the workspace memory runtime
- **THEN** the runtime SHALL use a Milvus-backed vector store factory
- **AND** it SHALL use the configured embedding provider
- **AND** startup logs SHALL identify the backend id and embedding provider without logging secrets

### Requirement: Chat completion SHALL persist bounded session memory through Memory Service

Successful chat sessions SHALL emit bounded session-shared memory through the Memory Service boundary so later active recall can retrieve durable task evidence.

#### Scenario: Successful chat writes scoped session memory

- **GIVEN** a chat session completes successfully
- **WHEN** the final session result is persisted
- **THEN** Macaca SHALL call `memory.remember` through `SystemMemoryClient`
- **AND** the command SHALL include application id, session id, agent name, and trace context
- **AND** the memory metadata SHALL identify automatic session completion capture without embedding application-specific business logic

### Requirement: Memory persistence failures SHALL be non-fatal and auditable

Memory persistence failure SHALL NOT turn an otherwise successful chat task into a failed chat task.

#### Scenario: Memory backend unavailable

- **GIVEN** a chat session completes successfully
- **AND** the configured memory backend is unavailable
- **WHEN** session memory capture is attempted
- **THEN** the chat session SHALL remain completed
- **AND** Macaca SHALL log a sanitized warning with session id, app id, agent name, and error class
- **AND** raw prompt or model output SHALL NOT be logged
