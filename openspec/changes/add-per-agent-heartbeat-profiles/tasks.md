# Tasks: Add Per-Agent Heartbeat Profiles

## 1. Contracts
- [x] 1.1 Extend app manifest heartbeat agent policy with cadence and gate fields.
- [x] 1.2 Extend Application Service heartbeat declaration view with native profile identity and safe policy fields.
- [x] 1.3 Extend Heartbeat profile summary/update DTOs with fixed interval and cooldown policy.

## 2. Runtime And Services
- [x] 2.1 Register one native Heartbeat profile per valid enabled manifest heartbeat agent.
- [x] 2.2 Copy profile metadata into native wakes for trace, dispatch filtering, and gate policy.
- [x] 2.3 Evaluate cooldown from profile policy with default fallback.
- [x] 2.4 Dispatch accepted per-agent wakes only to the matching declaration.

## 3. Web And Frontend
- [x] 3.1 Aggregate per-agent Heartbeat profiles and run histories by declaration scope keys.
- [x] 3.2 Let Heartbeat profile edits update fixed interval and cooldown separately.
- [x] 3.3 Show agent/profile/scope identity clearly in the Heartbeat Operations UI.

## 4. Validation
- [x] 4.1 Add/update focused tests for projection, provider policy, runtime dispatch, and Web aggregation.
- [x] 4.2 Run OpenSpec strict validation.
- [x] 4.3 Run Rust formatting, focused tests, and cargo checks.
- [x] 4.4 Run frontend lint/TypeScript checks.
- [x] 4.5 Run GitNexus detect changes and review affected flows.
