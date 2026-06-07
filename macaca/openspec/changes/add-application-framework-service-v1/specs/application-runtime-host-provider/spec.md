## ADDED Requirements

### Requirement: Runtime-Host Application Provider
The runtime host SHALL provide an `ApplicationSystemServiceProvider` that adapts existing Application Framework primitives into the provider-neutral Application Service.

#### Scenario: Provider delegates to application framework
- **WHEN** Application Service receives discover, start, stop, remove, status, snapshot, host dispatch, or GenUI commands
- **THEN** the runtime-host provider SHALL translate the service command into typed application commands and delegate to `macaca-app` framework primitives.

#### Scenario: Provider does not own application semantics
- **WHEN** the provider adapts `AppRegistry`, `AppRuntime`, ABI adapters, `ApplicationHost`, or GenUI runtime
- **THEN** application semantics SHALL remain in `macaca-app`, while runtime-host owns service lifecycle orchestration and command dispatch.

### Requirement: Provider Availability Behavior
The runtime-host Application Service provider SHALL return structured unavailable results when required runtime, registry, kernel compatibility handle, or host backend dependencies are absent.

#### Scenario: Missing runtime is unavailable
- **WHEN** a lifecycle command is dispatched and the provider has no configured application runtime
- **THEN** the provider SHALL return structured unavailable with service id, operation, trace id when available, and reason.

#### Scenario: One app failure does not block host startup
- **WHEN** one discovered application fails to start
- **THEN** the provider SHALL record diagnostics for that application and continue processing other applications where the command permits batch startup.

### Requirement: Provider Audit Logs
The runtime-host Application Service provider SHALL emit structured logs for provider start, discover, load, start, stop, remove, session, host dispatch, GenUI, failure, and snapshot emission.

#### Scenario: Lifecycle call is logged safely
- **WHEN** an application lifecycle command is processed
- **THEN** logs SHALL include service id, operation, trace id, application id/name when known, runtime kind, status, and counts, and SHALL NOT include prompt bodies, raw manifests, secrets, or raw host payloads.

#### Scenario: No business hardcode
- **WHEN** the provider selects a runtime adapter
- **THEN** it SHALL use runtime kind, ABI metadata, descriptor, or configuration, and SHALL NOT branch on application name, workflow name, package name, provider name, or business-specific names.

