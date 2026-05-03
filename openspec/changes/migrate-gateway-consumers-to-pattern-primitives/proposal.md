# Change: Migrate macaca-gateway consumers to pattern primitives

## Why

`macaca-gateway` now exposes builder, mediator, transport, message, and formatter primitives while keeping legacy `Gateway` / `ImAdapter` / `EventHandler` APIs deprecated but callable. Upper consumers should use the new primitives so deprecated gateway lifecycle APIs remain only as compatibility definitions inside `macaca-gateway`.

## What Changes

- Require production gateway consumers to start configured gateways through `GatewayBuilder`.
- Migrate gateway integration tests to `GatewayBuilder`, `GatewayMediator`, and `GatewayTransport`.
- Remove upper-crate calls to deprecated gateway lifecycle APIs.
- Keep deprecated gateway APIs available in `macaca-gateway` for compatibility and grep-based migration discovery.

## Impact

- Affected specs: `macaca-gateway-consumers`
- Affected code: `macaca-cli`, `macaca-integration-tests`
- Compatibility: legacy gateway APIs remain defined and callable inside `macaca-gateway`.
- Non-impact: no Telegram/Discord runtime behavior change, no web/session/chat_v2 integration, no new gateway platforms.
