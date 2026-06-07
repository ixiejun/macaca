## ADDED Requirements

### Requirement: WASM host imports use bounded command DTOs
Macaca SHALL represent every WASM host import as a bounded provider-neutral command with category, import name, optional target service id, operation, trace context, payload, and metadata.

#### Scenario: Service import command is built
- **WHEN** a guest requests `macaca:service/call`
- **THEN** the host import bridge SHALL build a typed command
- **AND** the command SHALL NOT expose concrete provider handles, backend clients, raw guest memory, or raw WASM bytes.

### Requirement: Host import validation is fail-closed
Macaca SHALL reject untraceable, oversized, unauthorized, or malformed host import commands before dispatching to guest code or service implementations.

#### Scenario: Missing trace is denied
- **WHEN** a host import command lacks trace context
- **THEN** the bridge SHALL return a structured denial with reason code `missing_trace`
- **AND** no ServiceRuntime call SHALL occur.
