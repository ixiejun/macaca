# Change: Preserve Context Service Trace Envelope

## Why
WASM application sessions can trigger Context Service assembly through `ServiceBackedContextClient`, but the SDK client currently serializes the typed command trace only inside the payload and drops the outer `ServiceCallCommand` trace envelope. The Web runtime service client then correctly rejects the call before dispatch, causing noisy `requires trace context` warnings and deprecated local assembler fallback.

## What Changes
- Preserve the typed Context command trace on the outer SDK `ServiceCallCommand`.
- Keep trace-required service admission intact; do not weaken policy, middleware, or Web shell boundaries.
- Add a regression test proving the context client forwards trace through the generic service boundary.

## Impact
- Affected specs: `context-service`
- Affected code: `macaca/crates/facade/macaca-sdk/src/context_client.rs`
