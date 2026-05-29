## 1. OpenSpec And Design

- [x] 1.1 Capture Superpowers brainstorm and write-plan in `docs/superpowers/plans`.
- [x] 1.2 Add OpenSpec proposal, design, and task checklist for application LLM model selection.
- [x] 1.3 Validate the OpenSpec change with strict validation.

## 2. LLM Service Catalog

- [x] 2.1 Extend `service.llm` catalog metadata to represent all configured providers, including sanitized unavailable rows.
- [x] 2.2 Ensure catalog and provider capability commands log trace id, scope, catalog size, and sanitized unavailable counts.
- [x] 2.3 Add service tests for multi-provider catalog output and unavailable provider diagnostics.

## 3. Route Resolution And Execution Override

- [x] 3.1 Thread request-level provider/model hints from `/api/chat/v2` into framework execution model selection.
- [x] 3.2 Persist requested and resolved route metadata for WASM execution sessions without storing prompts or provider payloads.
- [x] 3.3 Add route diagnostics for unavailable provider, unsupported model, default fallback, and accepted request override.

## 4. Application UI Bridge

- [x] 4.1 Expose catalog and route resolution through declared `service.call` bridge capabilities.
- [x] 4.2 Add bridge logs for catalog reads, route resolution, rejected undeclared calls, and execution starts.
- [x] 4.3 Add frontend checks for bridge payload shape and structured unavailable responses.

## 5. Codex WASM Workbench

- [x] 5.1 Replace free-text model entry with a provider/model selector sourced from backend `service.llm` data.
- [x] 5.2 Show selected route, default route, unavailable providers, and route diagnostics.
- [x] 5.3 Submit the selected provider/model hint when starting a real coding task.

## 6. Validation

- [x] 6.1 Run targeted Rust tests for affected LLM/runtime-host/web crates.
- [x] 6.2 Run frontend lint or focused bridge tests.
- [x] 6.3 Run a real Codex WASM Workbench task on Macaca OS and verify model selection evidence in SSE/session/audit output.
