# WASM Orchestration Portal Design

## Purpose

WASM applications should be first-class Macaca OS applications. They should be able to use task planning, app-scoped multi-agent delegation, Skill, MCP, LLM, service calls, and GenUI surfaces just like YAML applications, while retaining the flexibility to orchestrate those calls from guest code.

## Design

The implementation adds a generic WASM Orchestration Portal at the existing host import bridge. The bridge remains a Bridge/Adapter boundary: it validates guest imports, translates them into typed Command objects, routes them through ServiceRuntime or Application Service, bounds outputs, logs key execution points, and returns structured results.

Task operations use existing Task Service contracts. Agent delegation uses a new Application Service command backed by an injected orchestration backend. This backend is a Strategy supplied by the current host shell, so runtime-host does not depend on Web internals. Skill and MCP remain normal `service.call` targets and do not receive WASM-specific branches.

## Constraints

- No application-specific code in OS crates.
- No hardcoded business workflow, app name, symbol, provider, driver, or agent fallback.
- Every orchestration import requires trace and app/session scope.
- Every capability call is policy-governed and fail-closed.
- Logs and audit metadata include key ids and reason codes but never raw payload bodies, prompts, secrets, environment values, credentials, raw WASM bytes, or unbounded backend output.

## Expected Behavior

WASM apps without declared agents continue using the existing agentless dispatch path. WASM apps with declared agents still execute the WASM export, but the host prepares app-scoped executor/loops so the guest can create goals or delegate work during execution.
