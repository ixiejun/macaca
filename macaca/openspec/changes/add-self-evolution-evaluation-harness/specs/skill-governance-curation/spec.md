## ADDED Requirements

### Requirement: Macaca SHALL evaluate self-evolution with auditable white-box and black-box gates

The system SHALL provide a provider-neutral self-evolution evaluation contract
that proves governed skill evolution occurred and measures whether the evolved
skill state improves later real task execution.

#### Scenario: White-box evolution chain is complete
- **GIVEN** a task completed with verified terminal success evidence
- **WHEN** the evaluation provider scores a self-evolution run
- **THEN** the evaluation record SHALL include refs or bounded identifiers for
  the verified task completion, ExperienceCandidate, classification result,
  draft or patch proposal, curation dry-run, approval or promotion or apply
  decision, active catalog snapshot, and later skill read or activation
- **AND** the record SHALL include trace id, evidence refs, proposal id,
  curation run id, policy decision id when present, audit event ids, before and
  after snapshot refs, and rollback ref when present

#### Scenario: White-box gate rejects incomplete evolution
- **GIVEN** an evaluation record is missing proposal, curation, promotion,
  catalog visibility, or later activation evidence
- **WHEN** the evaluation provider scores the white-box gate
- **THEN** the result SHALL be `Failed` or `Inconclusive` with bounded failure
  reasons
- **AND** the system SHALL NOT claim that self-evolution completed

#### Scenario: Black-box evolved run improves without regressions
- **GIVEN** a generic task family has baseline and evolved run metrics
- **WHEN** the evaluation provider compares the runs
- **THEN** the evolved run SHALL preserve completion success and verified
  artifact quality
- **AND** the evolved run SHALL have no policy, audit, rollback, or
  sanitization regression
- **AND** at least one efficiency metric SHALL improve, including human
  intervention count, retry count, elapsed seconds, tool call count, or
  verified artifact density
- **AND** the evolved skill state SHALL have been read or activated

#### Scenario: Evaluation reports remain sanitized and generic
- **GIVEN** evaluation scoring has completed
- **WHEN** JSON or Markdown report refs are generated
- **THEN** reports SHALL include bounded metrics, checkpoint refs, pass/fail
  state, and failure reasons
- **AND** reports SHALL NOT include raw prompts, raw provider payloads, raw task
  output, secrets, credentials, package bytes, manifests, full skill bodies, or
  unbounded diagnostics
- **AND** reports SHALL NOT branch on application names, workflow names,
  provider names, model names, driver names, gateway names, chain names, or
  business domains

#### Scenario: Shells display evaluation results without owning semantics
- **GIVEN** a Web, CLI, or frontend surface displays a self-evolution
  evaluation
- **WHEN** the operator requests report state
- **THEN** the shell SHALL call SDK/SystemFacade evaluation commands or report
  readers
- **AND** the shell SHALL NOT implement scoring, checkpoint interpretation,
  curation semantics, promotion semantics, rollback semantics, or
  application-specific benchmark logic
