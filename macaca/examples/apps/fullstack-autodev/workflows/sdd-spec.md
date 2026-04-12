# SDD Spec Phase

You are in the **Spec** phase of Spec-Driven Development (SDD).

## Prerequisites

Analysis phase should already provide:
- Clear requirements
- Affected module scope
- Key architecture constraints

## Your Task

Produce an implementation-ready architecture handoff without writing feature code.

### 1. Consolidate Design Decisions

- Confirm system boundaries and ownership
- Define frontend/backend responsibilities
- Identify risk points and fallback strategy

### 2. Define Contracts

- API routes, request/response schema, and error model
- Data model changes and migration constraints
- Integration points with external services

### 3. Publish Handoff Docs

Use `file_write` to publish concise artifacts in `shared/`:

- `shared/architecture.md`
- `shared/api-contract.md`
- `shared/task-handoff.md`

Each file should be short, concrete, and directly actionable by implementation agents.

### 4. Dependency Graph

For each implementation task, define dependencies explicitly:

- Which tasks must be completed first
- Which contracts are blocking
- Which risks require validation before coding

## Guardrails

- Do not use `claude_code_execute` / `claude_code_resume` / `claude_code_status`
- Do not write production implementation code in this phase
- Keep output architecture-focused and execution-oriented

## Output Format

```markdown
## Spec Ready

### Architecture Summary
[1 paragraph]

### Interfaces
- [API/contract decision 1]
- [API/contract decision 2]

### Implementation Handoff
| Task | Assignee | Dependencies |
|------|----------|--------------|
| Task 1 | [worker] | - |
| Task 2 | [worker] | Task 1 |

### Risks And Constraints
- [risk 1 + mitigation]
- [constraint 1]

### Ready for Implementation
✓ Workers can start with clear contracts and dependencies
```
