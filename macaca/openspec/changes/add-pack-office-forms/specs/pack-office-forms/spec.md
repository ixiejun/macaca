## ADDED Requirements

### Requirement: Macaca SHALL expose Office Forms as a serviceized industrial pack

Macaca SHALL expose `pack.office.forms.v1` as a provider-neutral pack for form
provider inspection, form creation/import/opening, metadata inspection, schema
inspection, schema edit planning, schema edit requests, respondent session
creation, response draft validation, response submission, submission receipts,
response listing/retrieval, response export planning, response export requests,
event subscription planning, event subscription requests, event inspection,
artifact handles, health, snapshots, and replay diagnostics. The pack SHALL be
declared by applications, resolved by catalog/admission services, and invoked
only through descriptor-owned `forms.*` service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.office.forms.v1` as required and a forms provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy template hash, resource limits, approval rules, health metadata, compatibility metadata, and replay metadata
- **AND** SDK discovery SHALL expose callable `forms.*` commands without leaking credentials, webhook secrets, respondent PII, raw responses, raw exports, raw provider payloads, or provider secrets

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.office.forms.v1` as required but provider registration, host support, credential reference, permission, entitlement, resource, policy, validation, webhook support, or approval prerequisites are absent
- **THEN** admission SHALL block readiness with typed unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact a concrete provider, publish a form, mutate schema, submit a response, export responses, create a webhook, notify external systems, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.office.forms.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability memento
- **AND** SDK helpers and WASM ABI descriptors SHALL mark unavailable commands as non-callable while preserving structured diagnostics for application recovery

### Requirement: Office Forms commands SHALL use typed canonical service calls

Every `pack.office.forms.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace context, policy, resource, entitlement, approval, lifecycle, health,
snapshot, structured error, and audit behavior. SDK helpers, WASM ABI handlers,
application admission, web, CLI, and frontend code SHALL only build or submit
canonical service calls and SHALL NOT call forms providers directly.

#### Scenario: Provider capability is inspected
- **WHEN** `forms.inspect_provider` is invoked with declared scope and trace context
- **THEN** Macaca SHALL return sanitized provider capability metadata for create/open/import, schema editing, field types, validation rules, conditional logic, publish state, response validation, response submission, response read/export, webhook/watch support, auth, quota, lifecycle, health, and compatibility support
- **AND** the result SHALL include typed unavailable, unsupported, degraded, retired, schema-limited, validation-limited, logic-limited, response-limited, export-limited, webhook-limited, network-limited, and quota-limited states when applicable

#### Scenario: Form and response reads use bounded projections
- **WHEN** `forms.open_form`, `forms.inspect_metadata`, `forms.inspect_schema`, `forms.list_responses`, `forms.get_response`, `forms.inspect_events`, or `forms.get_artifact_handle` is invoked
- **THEN** Macaca SHALL enforce form, schema, field, response, respondent, export, event, artifact, permission, resource, and redaction scopes before provider access
- **AND** results SHALL be bounded, paged, partial, or asynchronous when needed, redacted according to policy, and represented by handles and summaries rather than raw response bodies, respondent PII, webhook payloads, raw exports, or unbounded response sets

#### Scenario: Unsupported command is requested
- **WHEN** a descriptor exists but the active provider does not support the requested `forms.*` command, field type, validation rule, conditional logic, publish feature, response feature, export format, webhook/watch mode, event type, or artifact mode
- **THEN** Macaca SHALL return a typed unsupported or schema-mismatch result with descriptor and capability diagnostics
- **AND** SDK discovery SHALL report the command or feature as non-callable for the current effective capability set

### Requirement: Office Forms DTOs SHALL be provider-neutral and hash-stable

`pack.office.forms.v1` SHALL define provider-neutral DTOs for `FormsScope`,
`FormsProviderCapability`, `FormHandle`, `FormMetadata`, `FormSchema`,
`FormSection`, `FormField`, `FormFieldOption`, `FormValidationRule`,
`FormLogicRule`, `FormPublishSettings`, `RespondentSession`,
`FormResponseDraft`, `FormResponseValue`, `FormSubmissionReceipt`,
`FormResponseExportPlan`, `FormEventSubscriptionPlan`, `FormEvent`, and
`FormArtifactHandle`. DTOs SHALL use stable handles, version hashes,
compatibility hashes, capability hashes, redaction classes, PII classes,
event cursors, and artifact handles rather than provider object references as
OS-layer semantics.

#### Scenario: Provider-specific concepts are mapped
- **WHEN** a provider exposes Google Forms item/question/response/watch objects, Typeform fields/responses/webhooks, Jotform forms/submissions/webhooks, Microsoft Forms unsupported gaps, or renderer-specific schema concepts
- **THEN** the provider adapter SHALL map those concepts into Macaca provider-neutral DTOs
- **AND** provider-specific extensions SHALL appear only as bounded `adapter_metadata` protected by capability hashes and SHALL NOT drive OS-layer routing

#### Scenario: Hashes preserve compatibility and replay
- **WHEN** Macaca serializes descriptors, provider capabilities, form versions, schema versions, field/validation/logic compatibility, response drafts, submission receipts, response export plans, event subscription plans, event cursors, artifact handles, and redaction profiles
- **THEN** it SHALL produce stable hashes suitable for compatibility checks, stale-version detection, validation diagnostics, audit correlation, and replay diagnostics
- **AND** schema evolution tests SHALL prove older compatible snapshots remain readable or return typed schema-mismatch diagnostics

### Requirement: Office Forms side effects SHALL use validation and plan/request separation

Macaca SHALL split schema edits, response exports, event subscriptions, and
other side-effecting forms operations into non-mutating plan or validation
commands and side-effecting request commands. Response submission SHALL require
respondent session state, schema version preconditions, validation diagnostics,
consent metadata, idempotency key, and submission receipt behavior.

#### Scenario: Schema edit plan validates before mutation
- **WHEN** `forms.plan_schema_edit` receives section, field, option, validation, conditional logic, publish setting, or response setting operations
- **THEN** Macaca SHALL validate operation schema, target handles, form version hash, schema version hash, field compatibility, validation support, logic support, publish policy, provider support, resource budget, redaction profile, and required approvals
- **AND** it SHALL return a plan with validation diagnostics without mutating the form, publishing the form, notifying respondents, or contacting external systems for side effects

#### Scenario: Response draft is validated before submission
- **WHEN** `forms.validate_response_draft` receives response values for a respondent session
- **THEN** Macaca SHALL validate schema version, field presence, required fields, validation rules, conditional visibility, answer types, PII classification, consent state, and resource limits
- **AND** it SHALL return validation diagnostics without submitting the response

#### Scenario: Response submit request executes idempotently
- **WHEN** `forms.submit_response_request` is invoked with a valid respondent session, schema version, validated draft, consent metadata, idempotency key, trace context, and sufficient permissions
- **THEN** Macaca SHALL submit through the forms service provider and return typed success, validation-failed, submit-denied, stale-version, conflict, response-redacted, approval-required, quota, timeout, cancellation, or failure results
- **AND** repeated requests with the same idempotency key SHALL NOT create duplicate submissions

#### Scenario: Export or event subscription request executes a validated plan
- **WHEN** `forms.response_export_request` or `forms.event_subscription_request` is invoked with a valid plan, retention policy, redaction profile, endpoint reference, signing policy, artifact scope, idempotency key, and approval state
- **THEN** Macaca SHALL return bounded artifact, subscription, or event handles
- **AND** raw response exports and webhook secrets SHALL remain outside trace, audit, snapshots, SDK diagnostics, and examples

### Requirement: Office Forms SHALL enforce permission, policy, resource, entitlement, and approval gates

Every `forms.*` command SHALL be scoped to application id, tenant id, session
id, task id, trace id, provider scope, form handle, schema handle, response
handle when applicable, actor or respondent handle when available, credential
reference, network policy, artifact policy, privacy profile, and permission
state. Side-effecting commands SHALL run policy, resource, entitlement,
approval, schema version, validation, webhook, and idempotency checks before
concrete provider calls.

#### Scenario: Permission is denied before provider access
- **WHEN** an application lacks `forms.provider.inspect`, `forms.form.create`, `forms.form.import`, `forms.form.open`, `forms.metadata.read`, `forms.schema.read`, `forms.schema.write`, `forms.response.session`, `forms.response.validate`, `forms.response.submit`, `forms.response.read`, `forms.response.export`, `forms.event.subscribe`, `forms.event.read`, or `forms.artifact.read`
- **THEN** Macaca SHALL return a typed denied result before invoking any provider
- **AND** audit evidence SHALL include bounded reason codes and sanitized scope handles only

#### Scenario: Sensitive operation requires approval
- **WHEN** a command touches public publish changes, PII collection, regulated answers, respondent identifiers, response exports, external webhook subscriptions, destructive schema edits, external notifications, or operations that publish artifacts or send data outside the tenant boundary
- **THEN** Macaca SHALL require approval when policy marks the operation approval-gated
- **AND** denial, expiration, or missing approval SHALL return typed approval-required diagnostics without side effects

#### Scenario: Resource or entitlement is unavailable
- **WHEN** form count, schema size, section count, field count, option count, validation rule count, logic rule count, response value count, response count, response export size, webhook event rate, artifact size, provider quota, network transfer, timeout, memory, storage, streaming output, retained snapshots, entitlement, or host support is insufficient
- **THEN** Macaca SHALL return typed quota, unavailable, denied, timeout, cancellation, or host-resource diagnostics
- **AND** the provider SHALL NOT be called for side-effecting operations after a failed gate

### Requirement: Office Forms responses, exports, webhooks, and artifacts SHALL be bounded and redacted

`pack.office.forms.v1` SHALL treat respondent identifiers, consent records,
response values, validation errors, response exports, webhook payloads,
webhook secrets, event payloads, and artifacts as sensitive data. The pack SHALL
expose handles, bounded summaries, cursors, redaction classes, PII classes,
retention metadata, and replay pointers rather than raw sensitive payloads in
observability surfaces.

#### Scenario: Response is read
- **WHEN** `forms.get_response` is invoked with sufficient permission
- **THEN** Macaca SHALL return bounded response value handles, field handles, validation state, PII classes, redaction classes, and retention metadata
- **AND** raw respondent PII, raw free-text answers, private identifiers, and unbounded response values SHALL NOT enter traces, audits, snapshots, or SDK diagnostics

#### Scenario: Response export is requested
- **WHEN** `forms.response_export_request` produces an export artifact
- **THEN** Macaca SHALL return artifact kind, source form/response scope, output format, size class, checksum handle, retention state, sensitivity class, and redaction class
- **AND** raw exported responses SHALL remain behind artifact boundaries

#### Scenario: Event subscription or webhook event is inspected
- **WHEN** `forms.event_subscription_request` or `forms.inspect_events` handles webhook/watch behavior
- **THEN** Macaca SHALL use endpoint references, signing policy handles, event cursors, event kinds, affected handle references, sanitized actor/respondent handles, timestamps, and redaction classes
- **AND** webhook secrets, raw webhook payloads, and provider-specific event bodies SHALL NOT become OS-layer event semantics

### Requirement: Office Forms SHALL preserve Macaca architecture boundaries

The Office Forms pack implementation SHALL preserve the microkernel, service
runtime, SDK/SystemFacade, application framework, runtime-host, plugin, and
shell boundaries defined by Macaca governance. Concrete forms providers SHALL
be replaceable Strategy adapters created only in approved runtime-host or
plugin composition roots.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, serviceization, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete Google Forms, Typeform, Jotform, Microsoft Forms, spreadsheet/export, notification, webhook transport, credential, or artifact provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.office.forms.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract, permission model, trace/audit schema, snapshot shape, and structured unavailable behavior
- **AND** OS layers SHALL NOT branch on provider names, form names, field names, webhook names, respondent names, application names, or workflow names

### Requirement: Office Forms SHALL emit sanitized trace, audit, health, snapshot, and replay evidence

`pack.office.forms.v1` SHALL emit sanitized declaration, admission,
provider-inspection, create/import/open, metadata-inspection, schema-inspection,
schema-edit, response-session, draft-validation, response-submission,
submission-receipt, response-list/read, response-export, event-subscription,
event-inspection, artifact-handle, policy, entitlement, resource, approval,
health, snapshot, unavailable, and failure events. Snapshots SHALL contain
enough bounded metadata to diagnose and replay service behavior without storing
raw sensitive content.

#### Scenario: Service call evidence is recorded
- **WHEN** any `forms.*` command is submitted
- **THEN** Macaca SHALL record trace-required service-call evidence with command name, descriptor version, sanitized scope handles, policy decision, resource decision, provider capability hash, result class, and replay pointer
- **AND** the evidence SHALL exclude raw credentials, webhook secrets, respondent PII, raw responses, raw exports, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded response sets

#### Scenario: Snapshot supports recovery diagnostics
- **WHEN** the service runtime records a forms snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, form/schema version hashes, command availability, provider health, policy template hash, resource counters, bounded schema/field/response/export/subscription summaries, event cursors, and sanitized replay pointers
- **AND** replay tests SHALL prove every `forms.*` command can be correlated through the canonical service path after restart

### Requirement: Office Forms SHALL provide industrial developer documentation

The implementation SHALL include a detailed developer guide at
`docs/developer-packs/office/forms.md` before `pack.office.forms.v1` is marked
complete. The guide SHALL be linked from SDK discovery metadata and the
industrial pack catalog index.

#### Scenario: Developer reads the guide
- **WHEN** a developer opens `docs/developer-packs/office/forms.md`
- **THEN** the guide SHALL explain purpose, manifest declaration, required versus optional behavior, permissions, provider scopes, form handles, metadata, schemas, sections, fields, options, validation rules, conditional logic, publish settings, respondent sessions, response drafts, response values, submission receipts, response exports, webhook/event subscriptions, event cursors, artifacts, unavailable diagnostics, provider replacement, operational limits, and conformance expectations
- **AND** it SHALL document every command DTO and result DTO with field-level behavior, idempotency, redaction, pagination, streaming/asynchronous export behavior, timeout, cancellation, approval, artifact retention, schema version preconditions, validation behavior, consent metadata, webhook signing policy, event renewal behavior, structured errors, and trace/audit interpretation

#### Scenario: Supplier mapping is documented
- **WHEN** the documentation describes supplier/API mapping
- **THEN** it SHALL map Google Forms API forms/items/responses/watches, Typeform Create/Responses/Webhooks APIs, Jotform forms/submissions/webhooks, Microsoft Forms/Graph capability gaps, webhook providers, and renderer schema concepts to Macaca abstractions
- **AND** it SHALL explicitly document what is intentionally not exposed as OS semantics

#### Scenario: Examples are provided
- **WHEN** the guide provides examples
- **THEN** examples SHALL use only synthetic forms, fields, validation rules, respondents, responses, exports, webhooks, events, artifacts, and unavailable diagnostics
- **AND** examples SHALL NOT include provider names, real credentials, webhook secrets, private respondent data, raw responses, raw exports, or workflow-specific conventions
