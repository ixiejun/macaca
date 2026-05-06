# Tasks

- [x] Scaffold OpenSpec proposal/design/spec delta
- [x] Extend `macaca-proto` `ContextConfig` with `provider_families` and `trust_governance`
- [x] Add `ContextFacadeAssemblyPolicy` + integrate trust pass in governance pipeline
- [x] Implement `catalog::assemble_context_providers` with neutral family ids
- [x] Add `ProviderFamilyDescriptor` + `ContextProviderFactory::descriptor`
- [x] Add `implementation_version` on `ContextProvider` trait (default `None`)
- [x] Add `ProviderHealthLedger` + HTTP route `GET /api/context/provider-runtime`
- [x] Add `OpaqueExternalPayload` validation module
- [x] Wire `RuntimeConfig` with `context` for assembler in `agentic_loop`
- [x] Refactor `ContextReportingChatModel` to use catalog assembler
- [x] `cargo test` relevant crates + `openspec validate --strict`
