# Change: Fix crypto signal host aggregation

## Why
The WASM crypto signal app currently reports a policy-shaped failure for the initial market data import and the final LLM analysis does not receive delegated technical/risk evidence when earlier host-command results include fail-closed entries.

## What Changes
- Classify WASM host import service failures separately from policy denials.
- Add a generic live crypto market-data fallback source so finance domain-pack market snapshots remain available when the primary exchange endpoint fails.
- Update the crypto signal guest pipeline to reference the actual successful delegated host-command result indexes.
- Remove host-side prompt parsing for finance symbols; WASM apps must provide typed `symbol`/`ticker` fields before crossing the service boundary.

## Impact
- Affected specs: session-genui-crypto-signal-app
- Affected code: macaca-runtime-host finance domain pack and WASM host import bridge; generic WASM component template binding; standalone wasm-crypto-signal-app guest metadata.
