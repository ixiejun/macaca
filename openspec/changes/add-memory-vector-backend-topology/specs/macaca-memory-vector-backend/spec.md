## ADDED Requirements

### Requirement: Macaca SHALL provide a VectorMemoryBackend topology contract

Macaca SHALL define a supplier-neutral `VectorMemoryBackend` contract for long-term vector memory topology. The contract SHALL preserve the architecture concept that an application maps to a vector database or equivalent isolation domain and each agent maps to a collection/table/partition or equivalent isolation unit.

#### Scenario: Topology is supplier neutral

- **GIVEN** a vector backend implementation is selected
- **WHEN** Macaca resolves vector memory topology
- **THEN** it SHALL resolve an application-level isolation domain
- **AND** it SHALL resolve an agent-level isolation unit for agent private memory
- **AND** the implementation MAY use database/collection/table/namespace/partition terminology internally

#### Scenario: Existing VectorStore remains compatible

- **GIVEN** existing code depends on `VectorStore`
- **WHEN** `VectorMemoryBackend` is added
- **THEN** `VectorStore` SHALL remain available
- **AND** `VectorMemoryBackend` SHALL operate as a higher-level topology contract rather than deleting `VectorStore`

### Requirement: Milvus SHALL be the default vector memory backend

Macaca SHALL provide Milvus as the default long-term vector memory backend.

#### Scenario: Application maps to Milvus database

- **GIVEN** an application id is available
- **WHEN** the default Milvus backend prepares vector memory
- **THEN** it SHALL create or select a Milvus database corresponding to that application id
- **AND** all default private agent collections for that application SHALL live under that database

#### Scenario: Agent maps to Milvus collection

- **GIVEN** an application database exists
- **AND** an agent id or agent name is available
- **WHEN** the default Milvus backend prepares agent private vector memory
- **THEN** it SHALL create or select one collection corresponding to that agent
- **AND** default agent private search SHALL be limited to that collection

#### Scenario: Session shared memory uses explicit shared collection

- **GIVEN** memory visibility is `SessionShared`
- **WHEN** the default Milvus backend stores or searches shared vector memory
- **THEN** it SHALL route to an explicit session/project shared collection
- **AND** it SHALL NOT mix shared records into an agent private collection

### Requirement: Vector records SHALL carry scope and provenance

Vector memory records SHALL include enough structured data for recall, governance, trace, and deletion.

#### Scenario: Record includes required fields

- **GIVEN** a memory record is inserted into vector memory
- **WHEN** the backend persists the vector payload
- **THEN** the record SHALL include memory id, scope, visibility, content, vector, metadata, created time, and source/provenance
- **AND** the record SHOULD include updated time, confidence, freshness, and conflict metadata when available

#### Scenario: Search hit returns provenance

- **GIVEN** vector search returns memory hits
- **WHEN** the facade or active recall consumes the hits
- **THEN** each hit SHALL expose memory id, score, scope, visibility, snippet/content summary, and provenance metadata

### Requirement: Replacement vector backends SHALL prove topology equivalence

Any non-Milvus vector backend used as a default long-term vector memory backend SHALL support topology equivalent to application database plus agent collection.

#### Scenario: Replacement backend maps application and agent isolation

- **GIVEN** a replacement backend such as LanceDB, Qdrant, or remote vector service is configured
- **WHEN** it registers as a default vector memory backend
- **THEN** it SHALL provide a mapping from application id to an isolation domain
- **AND** it SHALL provide a mapping from agent id/name to an isolation unit within that domain

#### Scenario: Flat namespace backend is rejected for default vector memory

- **GIVEN** a backend only supports a single flat namespace without enforceable application and agent isolation
- **WHEN** it attempts to register as a default long-term vector memory backend
- **THEN** Macaca SHALL reject it or mark it unavailable for default vector memory
- **AND** it MAY still be used as a supplement or remote RAG adapter if explicitly configured

#### Scenario: Conformance tests prevent cross-agent leakage

- **GIVEN** a vector backend implementation participates in conformance tests
- **WHEN** agent A and agent B store private vector records under the same application
- **THEN** agent A private search SHALL NOT return agent B records
- **AND** agent B private search SHALL NOT return agent A records

### Requirement: Vector backend status SHALL be observable

Macaca SHALL expose vector backend diagnostics sufficient for development, testing, and trace/debug UI.

#### Scenario: Status reports topology

- **GIVEN** a vector backend is initialized
- **WHEN** status is requested
- **THEN** the status SHALL include backend id, application isolation domain, selected collection/table/partition, dimension, availability, and last error when present

#### Scenario: Backend failure degrades gracefully

- **GIVEN** vector backend setup or search fails
- **WHEN** memory search continues through other layers
- **THEN** Macaca SHALL record the vector error in diagnostics
- **AND** it SHALL not crash the agent run solely because vector search is unavailable
