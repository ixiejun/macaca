## ADDED Requirements

### Requirement: WASM runtime diagnostics are sanitized

Macaca SHALL keep WASM runtime diagnostics bounded, provider-neutral, and safe for logs, traces, SDKs, and UI surfaces.

#### Scenario: Diagnostics exclude sensitive raw data
- **WHEN** runtime availability, session creation, session rejection, or command rejection diagnostics are produced
- **THEN** diagnostics SHALL NOT contain raw WASM bytes, raw command payloads, raw manifests, secrets, environment values, API keys, private keys, prompts, or unbounded provider output.

#### Scenario: Diagnostics remain traceable
- **WHEN** a runtime provider reports unavailable, disabled, rejected, or unsupported state
- **THEN** diagnostics SHALL include a reason code, bounded message, runtime kind, and trace id when trace context is available.
