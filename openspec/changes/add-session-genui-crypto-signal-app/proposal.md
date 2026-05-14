# Change: Add Session GenUI Crypto Signal App

## Why

Macaca can already host WASM applications, route generic `service.call` imports, and render controlled GenUI component trees in the Web Shell. The next app shape needs to prove a different product mode from the stock app: a chat-first WASM application that keeps the main thread, session stream, composer, and AgentPanel while rendering analysis output as host-owned GenUI cards.

## What Changes

- Add a manifest-declared `builtin_kit` session surface contract for applications that do not ship app-owned web bundles.
- Connect WASM/session execution output to a queryable GenUI surface without adding application-specific frontend branches.
- Create an independent `/Users/quantum/Code/dev/wasm-crypto-signal-app` repository that declares `pack.finance.v1` with crypto modeled as a finance `asset_class`, uses generic host `service.call`, and emits card/table/list/badge-style GenUI intent data.
- Preserve the stock app boundary: stock remains an application-surface web bundle, while crypto remains a session-surface built-in-kit app.
- Keep all market, news, and LLM access behind host services; the guest never owns provider secrets or direct network access.

## Impact

- Affected specs: application-session-genui-runtime, existing GenUI runtime behavior, existing application UI runtime behavior.
- Affected code: `macaca-app` manifest UI model/admission, `macaca-proto` application/GenUI DTOs if a storage/query envelope is missing, `macaca-runtime-host` Application Service GenUI surface provider, `macaca-web` chat orchestration and GenUI routes, `frontend/app/chat/[appId]/page.tsx`, and the new independent crypto app repository.
- Compatibility: existing YAML apps, stock app web-bundle application surface, `/api/chat/v2`, app-owned UI bridge, service-call audit replay, and GenUI renderer behavior must keep working without application-id branches.
