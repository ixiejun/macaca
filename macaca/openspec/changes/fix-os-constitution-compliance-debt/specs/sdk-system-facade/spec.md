## ADDED Requirements

### Requirement: Domain-Pack Clients Share A Generic Preflight Builder Skeleton

SDK domain-pack clients SHALL share a generic, provider-neutral preflight/command
builder skeleton so per-domain clients do not linearly duplicate scaffolding. A
bespoke per-domain client SHALL be justified only by domain-specific high-risk
write semantics; otherwise the generic builder SHALL be used.

#### Scenario: New domain reuses the generic builder
- **WHEN** a new domain-pack client is added without domain-specific write risk
- **THEN** it SHALL reuse the generic preflight/command builder rather than copy a
  bespoke per-domain builder

#### Scenario: Client tests are table-driven
- **WHEN** domain-pack client catalog behavior is tested across packs
- **THEN** the tests SHALL be table-driven over the pack set rather than
  copy-pasted per pack
