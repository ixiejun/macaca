# Change: Add Application Sanitized Metadata Service

## Why

Web and upper shells still read raw application manifests for entry agents, agent lists, tool policy, context policy, skill/MCP overlays, and runtime metadata. That keeps presentation shells involved in Application Framework semantics and risks leaking prompt/config data. Application Service needs a sanitized metadata view so shells can remain adapters.

## What Changes

- Add Application Service metadata query commands and sanitized metadata views.
- Project Manifest v1 / AgentAbility / YAML compatibility data into safe views for Web, CLI, Gateway, framework runner, and toolkit assembly.
- Migrate Web call paths to prefer `SystemApplicationClient` metadata views before deprecated raw manifest fallback.
- Keep existing `/api/chat/v2`, session trace, goal resume, toolkit, skill/MCP overlay behavior unchanged.

## Impact

- Affected specs: `application-sanitized-metadata-service`
- Affected code:
  - `macaca/crates/foundation/macaca-proto/src/application_service.rs`
  - `macaca/crates/application/macaca-app/src/service_adapter.rs`
  - `macaca/crates/application/macaca-app/src/service_projection.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/application_service_provider.rs`
  - `macaca/crates/facade/macaca-sdk/src/application_client.rs`
  - `macaca/crates/shells/macaca-web/src/routes.rs`
  - `macaca/crates/shells/macaca-web/src/chat_orchestrator.rs`
  - `macaca/crates/shells/macaca-web/src/framework_runner.rs`
  - `macaca/crates/shells/macaca-web/src/framework_toolkit.rs`
  - `macaca/crates/shells/macaca-web/src/loop_manager.rs`
  - `macaca/crates/shells/macaca-web/src/skill_mcp.rs`
- Depends on: `add-application-manifest-v1-ability-baseline`, `migrate-yaml-apps-to-manifest-v1-agent-ability`
