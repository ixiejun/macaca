## 1. Service Contract
- [x] 1.1 Add `ApplicationMetadataQueryCommand`.
- [x] 1.2 Add sanitized views for application, ability, entry, tool policy, context policy, skill policy, MCP overlay, and manifest digest.
- [x] 1.3 Extend `SystemApplicationClient` with metadata query methods.

## 2. Projection
- [x] 2.1 Add `service_projection` module in `macaca-app`.
- [x] 2.2 Project Manifest v1 and YAML AgentAbility data to sanitized views.
- [x] 2.3 Add redaction tests for prompt, raw manifest, agent config, env, secret, API key, and host payload fields.

## 3. Web Migration
- [x] 3.1 Run GitNexus impact before editing Web symbols that read application manifests.
- [x] 3.2 Migrate app routes and chat preflight to service-first metadata views.
- [x] 3.3 Migrate framework runner/toolkit/skill-MCP overlay reads where safe. Current remaining raw reads are compatibility-only execution inputs for prompt/model/context/skill allow-deny/MCP definitions, which are intentionally excluded from sanitized metadata views.
- [x] 3.4 Keep deprecated raw manifest fallback with explicit expiry notes.

## 4. Validation
- [x] 4.1 Run Web/application regression tests.
- [x] 4.2 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 4.3 Run `cargo test -p macaca-integration-tests route_c_workspace_topology`.
- [x] 4.4 Run `cargo check --workspace`.
- [x] 4.5 Run `npx gitnexus detect-changes -r agent`.
