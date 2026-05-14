## Context

The stock analyst app intentionally owns the center workspace through `ui.runtime: web_bundle` and `ui.surface.mode: application`. The crypto signal app is a different application class: it should keep Macaca's main chat thread and use the host's built-in GenUI kit for the analysis result. Existing code already provides most platform pieces: manifest service contracts, app-scoped WASM sessions, replayable `service.call` audit, `/api/chat/v2` stream normalization, and a frontend `GenUiRenderer`. The missing generic bridge is that `APPLICATION_GENUI_SURFACE_COMMAND` currently returns unavailable, so session-produced GenUI intent cannot become the card stack that the Web Shell already knows how to render.

## Goals

- Build a standalone `wasm-crypto-signal-app` that accepts ticker-like crypto symbols such as `BTC`, `ETH`, and `SOL`.
- Keep the app chat-first by declaring `ui.runtime: builtin_kit` and `ui.surface.mode: session`.
- Render analysis as controlled GenUI cards using supported component kinds, including `card`, `table`, `list`, and text/badge-like metadata.
- Route crypto market data, crypto news, and LLM analysis through generic `service.call` with replayable audit evidence.
- Add only generic platform support that any session-surface WASM app can reuse.

## Non-Goals

- Do not add crypto-specific branches to `macaca-web`, the frontend, the runtime host, or the application framework.
- Do not implement a crypto provider with real external credentials in this change; unavailable or mock host services must remain structured and auditable.
- Do not let the guest perform direct networking or store API keys.
- Do not present strong financial advice. The app output is signal and risk analysis with explicit `not_financial_advice` metadata.
- Do not replace the existing GenUI renderer with a custom crypto renderer.

## Design Decisions

### 1. Use Session Surface + Builtin GenUI Kit

The manifest should separate the loading strategy from the placement strategy. `builtin_kit` means the app emits declarative GenUI intent data instead of shipping a web bundle. `surface.mode: session` means the host chat workspace remains primary. This reuses the Strategy pattern already present in the frontend renderer: the shell dispatches by component kind, not by app id or domain.

### 2. Persist Latest Session GenUI Intent Behind Application Service

WASM dispatch should treat `ApplicationImport::UiRender` as a generic host import category. When a guest emits a render command, the runtime host validates the payload as `UiIntent` or a bounded render envelope, stores it by `(app_id, session_id, surface_id)`, and returns a structured result. `APPLICATION_GENUI_SURFACE_COMMAND` then reads that store. This follows Facade + Repository: Web routes query Application Service, and Application Service hides storage and runtime details.

### 3. Keep Crypto Logic in the Independent Guest

The crypto app repository should mirror the stock app's clean boundary but omit React UI packaging. The guest owns input normalization, deterministic risk/signal construction, metadata host-command declarations, and artifact packaging. Host services own market/news/LLM data, policy, secrets, and audit.

### 4. Represent Cards as Data, Not UI Code

The app should emit a GenUI tree whose root is a `card`. Children may include text, table, and list components. A badge visual can be represented with card/list/table metadata first; a first-class `badge` component can be added later as a general GenUI extension if needed. This avoids blocking the app on renderer expansion.

### 5. Chain Declared Host-Command Results Generically

The portable Component Model adapter still reads deterministic host-command metadata instead of executing guest bytecode directly. To model real guest orchestration without app-specific runtime code, declared command payloads may reference previous command results through bounded placeholders such as `${host.results.0.output}` and `${host.results.2.output.analysis}`. The runtime resolves those placeholders immediately before each command dispatch. This keeps the guest responsible for workflow composition while Macaca remains a generic OS boundary that only routes services, policy, audit, and UI storage.

## Data Flow

1. User sends `分析 BTC 买卖信号` in the existing Web chat composer.
2. `/api/chat/v2` detects the app-scoped WASM runtime path and dispatches `app:start` through Application Service.
3. The component-model session invokes the app's declared host-command plan.
4. The guest plan calls `service.market_data`, `service.news_digest`, and `service.llm.analysis` through `service.call`, with crypto represented as a finance `asset_class` in payload metadata.
5. The runtime resolves declared result placeholders so the analysis command receives the market and news outputs as structured evidence.
6. The guest plan emits `macaca:ui/render` with a bounded GenUI intent whose card content is populated from the analysis result.
7. The frontend receives the normalized SSE assistant/done flow and then queries `/api/apps/{appId}/genui/surface?session_id=...`.
8. `GenUiRenderer` mounts the returned component tree above or within the conversation stack, while the right AgentPanel and trace/audit panels remain available.

## Error Handling

- Missing or invalid symbols are represented as user-visible GenUI card content plus structured host metadata, not panics.
- Missing crypto host services return structured unavailable service-call results and remain visible in audit replay.
- Missing render trace or session scope rejects the GenUI render command before storage.
- Unsafe or unknown UI components remain blocked by existing GenUI renderer validation and unsupported placeholders.

## Testing Strategy

- Add manifest/admission tests for `ui.runtime: builtin_kit` with `surface.mode: session`.
- Add Application Service tests proving `UiRender` stores a bounded session surface and `APPLICATION_GENUI_SURFACE_COMMAND` replays it by app/session/surface.
- Add WASM component-provider tests proving declared `ui.render` host commands are routed without using `service.call` policy shortcuts.
- Add frontend tests or type-level checks proving session GenUI surfaces still render without app-id branches.
- Add crypto app contract tests for symbol normalization, disclaimer metadata, manifest service ids, component marker retention, and package install layout.

## Risks and Mitigations

- Risk: `UiRender` support could become another presentation-owned shortcut. Mitigation: route it through Application Service and protocol DTOs, not frontend-specific code.
- Risk: crypto service ids may drift from future domain pack names. Mitigation: keep ids in manifest/contract constants inside the app repo and use service-contract expansion diagnostics.
- Risk: card output becomes financial advice. Mitigation: enforce `analysis_only` and `not_financial_advice` metadata in guest payloads and rendered text.
- Risk: current portable component adapter only reads metadata commands. Mitigation: use the same deterministic host-command marker approach as the stock app until a real component engine is wired behind the adapter.
