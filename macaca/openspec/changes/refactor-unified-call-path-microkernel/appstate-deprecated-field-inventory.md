# AppState deprecated direct-provider field inventory — P3 §4.1.1

Inventory of deprecated `AppState` provider anchors in `macaca-web/src/state.rs` and their
production read/write sites before P3 thin-shell migration.

## Deprecated fields

| Field | Type | Replacement SDK client | Migration adapter |
|-------|------|------------------------|-------------------|
| `runtime` | `Arc<AppRuntime>` | `SystemApplicationClient` (`discover`, `status`, `start`, `app_agents` via metadata) | `application_shell_adapter` |
| `registry` | `Arc<RwLock<AppRegistry>>` | `SystemApplicationClient::metadata` (sanitized view; raw manifest fallback only in adapter) | `application_shell_adapter` |
| `llm` | `Arc<dyn LlmProvider>` | `SystemLlmClient::snapshot` | `llm_route_shell_adapter` |
| `llm_router` | `Arc<LlmRouter>` | `SystemLlmClient::resolve_route` | `llm_route_shell_adapter` |
| `memory_runtime` | `Option<Arc<WebMemoryRuntime>>` | `SystemMemoryClient` | composition bundle (bootstrap-only; tools use injected runtime handle) |
| `mcp_runtime` | `Arc<McpRuntimeFacade>` | `SystemMcpClient` | `mcp_shell_adapter` |
| `driver_registry` | `Arc<DriverRegistry>` | `SystemDriverClient` | composition bundle (bootstrap-only) |
| `driver_runtime` | `Arc<DriverRuntime>` | `SystemDriverClient` | composition bundle (bootstrap-only) |

## Production read sites (pre-migration)

### `runtime` (`state.runtime`)

| File | Usage |
|------|-------|
| `routes.rs` | `list_apps`, `app_agents`, status fallback app count |
| `lib.rs` | bootstrap wiring only |

### `registry` (`state.registry`)

| File | Lines (approx) | Usage |
|------|----------------|-------|
| `routes.rs` | 8+ | entry agent fallback, app list/detail/agents/reload |
| `chat_orchestrator.rs` | 4 | manifest llm_config, wasm layer, agent manifests |
| `framework_runner.rs` | 6 | llm_config, context config, agent manifests |
| `framework_toolkit.rs` | 3 | manifest reads for toolkit construction |
| `loop_manager.rs` | 2 | plan loop consumer manifest reads |
| `skill_mcp.rs` | 2 | skill snapshot registry reads |
| `skill_self_evolution_audit.rs` | 1 | diagnostic registry comparison |
| `app_ui_routes.rs` | 1 | manifest path fallback |

### `llm` / `llm_router`

| File | Field | Usage |
|------|-------|-------|
| `routes.rs` | `llm` | `GET /api/status` provider label |
| `chat_orchestrator.rs` | `llm_router` | `resolve_request_route_metadata` |
| `framework_runner.rs` | `llm_router` | `resolve_model_selection` |

### `mcp_runtime`

| File | Usage |
|------|-------|
| `skill_mcp.rs` | `register_definitions` for skill MCP tool registration |

### `memory_runtime` / `driver_*`

| File | Usage |
|------|-------|
| `lib.rs` | bootstrap composition root only |
| `context_memory_tools.rs` | tool struct holds `WebMemoryRuntime` directly (not `AppState` field) |

## Structural target (P3 §4.1.3)

- Remove deprecated fields from `AppState`.
- Introduce `WebShellCompositionBundle` holding provider anchors for bootstrap and approved adapter fallback only.
- Route/session code uses `application_shell_adapter`, `llm_route_shell_adapter`, `mcp_shell_adapter`.
- `serviceization_escape_hatches` allowlist shrinks to adapter modules + composition bundle definition.

## Notes

- `ApplicationMetadataView` does not expose full manifests (`llm_config`, full `context`); registry fallback remains in adapters until Application Service metadata expands.
- GitNexus HIGH/CRITICAL warnings on this refactor are recorded in `impact-memo.md` and do not block P3.
