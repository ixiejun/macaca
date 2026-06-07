# Tasks — integrate-context-memory-skills-mcp (umbrella)

## Spec

- [x] Add `specs/integration-acceptance/spec.md` with ADDED requirements + scenarios
- [x] `openspec validate integrate-context-memory-skills-mcp --strict`

## Implementation (closed by this change set)

- [x] `CompiledContext` type alias + `CapabilityCandidate` alias
- [x] Composer stable/dynamic candidate fingerprints on `ComposerPlanSummary`
- [x] Profile YAML frontmatter strip + content scan + heartbeat skip test
- [x] `WorkspaceMemoryRecallSource` tombstone filter + web wiring
- [x] `MergedTombstoneIndex` in `macaca-memory`
- [x] Combined digest/tombstone unit coverage + `OpaqueExternalPayload` adapter test
- [x] `docs/context-facade-call-sites.md` inventory
- [x] `macaca-context` README external boundary note

## Follow-ups (optional / not blocking umbrella)

- [ ] Remote `ContextProvider` RPC protocol
- [ ] Proto-level `ProfileContentScanner` injection hook (trait object from web)
