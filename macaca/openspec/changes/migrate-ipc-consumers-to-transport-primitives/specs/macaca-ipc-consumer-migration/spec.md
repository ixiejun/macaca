## ADDED Requirements

### Requirement: Upper-layer IPC consumers SHALL migrate away from deprecated bus sender/receiver entry points

Upper-layer crates SHALL stop calling deprecated `LocalBus::{sender,receiver}` and `NatsBus::{sender,receiver}` APIs once an equivalent transport bridge entry is available.

#### Scenario: Kernel test migrates from deprecated local sender
- **WHEN** `macaca-kernel` constructs an IPC sender for `IpcServiceAdapter`
- **THEN** it uses the transport bridge sender creation path instead of `LocalBus::sender()`
- **AND** the message delivery behavior remains unchanged

### Requirement: Kernel IPC adapter SHALL consume bridge-compatible sender abstraction

`macaca-kernel::IpcServiceAdapter` SHALL accept the dynamic sender abstraction exported by `macaca-ipc`, so it can operate against the transport bridge contract rather than concrete sender generics.

#### Scenario: Adapter wraps dynamic sender
- **WHEN** a caller passes a `DynMessageSender` to `IpcServiceAdapter`
- **THEN** the adapter forwards `IpcService::send` through that sender
- **AND** no caller needs to know the concrete transport sender type

### Requirement: Consumer migration SHALL not invent transport-factory usage without a real selection path

Upper-layer migration SHALL only introduce `IpcTransportFactory` where a real transport selection need exists; otherwise migration stops at the bridge-compatible sender boundary.

#### Scenario: No real transport selection path exists
- **WHEN** the audited upper-layer code only needs a concrete local sender for tests or fixed wiring
- **THEN** the migration uses `create_sender()` directly
- **AND** does not introduce a fake configuration or factory call solely to satisfy the abstraction
