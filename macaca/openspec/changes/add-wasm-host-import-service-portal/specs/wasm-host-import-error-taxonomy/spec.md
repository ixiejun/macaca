## ADDED Requirements

### Requirement: WASM host import errors use stable reason codes
Macaca SHALL map host import bridge failures into stable reason codes that are independent of concrete services or engine implementations.

#### Scenario: Error mapping is stable
- **WHEN** a host import fails because of missing trace, missing capability, payload too large, service unavailable, policy denial, unsupported import, or service failure
- **THEN** the result SHALL include one of `missing_trace`, `capability_missing`, `payload_too_large`, `service_unavailable`, `policy_denied`, `unsupported_import`, or `service_failed`.
