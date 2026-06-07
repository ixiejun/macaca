## 1. Implementation

- [x] 1.1 Read the three required governance documents and confirm the supply-chain gate stays in proto/app/sdk fixture boundaries without kernel/Web/CLI provider ownership.
- [x] 1.2 Run GitNexus impact analysis for `WasmPackageAdmissionSpec`, `WasmPackageAdmissionReport`, `WasmComponentArtifactDescriptor`, and `ApplicationCertificationFixture`.
- [x] 1.3 Add failing proto tests for signed artifact metadata serialization and sanitization.
- [x] 1.4 Add `wasm_supply_chain.rs` DTOs for signature, signer, provenance, origin, trust policy, and verification report.
- [x] 1.5 Add admission tests for accepted signed artifact, missing signature, digest mismatch, untrusted signer, stale provenance, origin mismatch, and incompatible certification.
- [x] 1.6 Add `wasm_supply_chain.rs` admission Specification in `macaca-app`.
- [x] 1.7 Integrate verification into `WasmPackageAdmissionSpec`.
- [x] 1.8 Add SDK package fixtures for signed and rejected artifacts.
- [x] 1.9 Add English comments explaining DTO safety, deterministic verifier strategy, and sanitization rules.
- [x] 1.10 Add sanitized logs for supply-chain verification decisions.
- [x] 1.11 Run `cargo test -p macaca-proto wasm_supply_chain --manifest-path macaca/Cargo.toml`.
- [x] 1.12 Run `cargo test -p macaca-app wasm_supply_chain --manifest-path macaca/Cargo.toml`.
- [x] 1.13 Run `openspec validate add-wasm-artifact-supply-chain-verification --strict`.
- [x] 1.14 Run GitNexus detect changes before commit.
