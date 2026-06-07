## Context
This is a narrow bug fix for the existing WASM crypto signal app and finance domain-pack runtime path. The design uses existing Bridge and Adapter boundaries: host imports remain generic service calls, live market-data providers remain behind the finance pack, and the app guest only declares result references.

## Decisions
- Keep fail-closed semantics for unavailable live data, but try a second public no-key exchange adapter before returning outage.
- Preserve deny-by-default policy behavior; only change the status category used for service failures.
- Keep aggregation inside the standalone app guest metadata, because host-command result indexes are part of the app-declared pipeline, not Macaca OS business logic.
- Treat finance symbol extraction as schema validation only. Macaca accepts typed `symbol` or `ticker` fields, rejects raw prompts as `InvalidArgument`, and never converts natural-language text or market pairs into base assets.
- Let Component Model metadata bind exact structured chat fields such as `${chat.symbol}`. This is a generic JSON lookup; the app or app-owned coordinator must create the field.
