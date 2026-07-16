# Office Forms Pack Design

## Context

`pack.office.forms.v1` exposes form operations as a Macaca OS serviceized
capability. It lets applications create, inspect, validate, submit, export, and
observe forms without embedding Google Forms, Typeform, Jotform, Microsoft
Forms, survey-rendering libraries, spreadsheet exports, webhook transports, or
application-specific collection workflows into generic OS layers.

Forms are both user interfaces and data collection contracts. They contain
question schemas, validation rules, conditional logic, publish state,
respondent sessions, submission receipts, response exports, and event streams.
The pack therefore separates schema edits, response validation/submission, and
response export into typed, audited command flows with redaction and
idempotency.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Google Forms API | Create/get forms, batchUpdate schema edits, publish settings, response get/list, watches for schema and response events | Form handle, schema edit plan, publish settings, response cursor, event subscription |
| Typeform APIs | Create/update forms, themes/images, retrieve responses, webhooks for real-time submissions | Form schema/field DTOs, response export plan, webhook subscription plan, event cursor |
| Jotform API | Account/form/submission access, submission download patterns, webhooks | Provider capability, form metadata, submission/response DTOs, webhook event adapter |
| Microsoft Graph / Microsoft Forms gap | Enterprise auth/paging patterns but no generally available Forms response API in Graph | Provider capability can report unsupported/unavailable response access rather than fake compatibility |
| HTML/JSON Schema style renderers | Field types, validation, required flags, conditional visibility | Provider-neutral schema/field/validation/logic DTOs and conformance fixtures |

The pack exposes provider-neutral contracts. Provider adapters translate to
cloud forms APIs, hosted survey providers, local schema/render engines,
webhook/event bridges, spreadsheet/export providers, or unavailable providers.
OS layers must not branch on provider names, form titles, field labels,
respondent identities, webhook tags, or business workflows.

## Goals

- Provide stable pack id `pack.office.forms.v1` and command namespace
  `forms.*`.
- Support provider inspection, form create/import/open, metadata inspection,
  schema/field/validation/logic/publish inspection, schema edit planning,
  schema edit requests, respondent session creation, response draft validation,
  response submission, submission receipt retrieval, response listing/retrieval,
  response export planning/requests, webhook/event subscription planning/
  requests, event inspection, artifact handles, snapshots, health, and replay
  diagnostics.
- Preserve safety with form/response/artifact scopes, PII classification,
  consent metadata, validation rules, idempotent submission, webhook signature
  verification, export retention, approvals, quotas, bounded response paging,
  and sanitized audit.
- Keep concrete form providers behind replaceable service providers.
- Require developer documentation at `docs/developer-packs/office/forms.md`.

## Non-Goals

- Do not implement concrete Google Forms, Typeform, Jotform, Microsoft Forms,
  spreadsheet export, notification, webhook transport, or form renderer
  providers in this proposal.
- Do not define survey, HR, medical, legal, school, CRM, quiz, order, lead-gen,
  consent, or workflow-specific business logic.
- Do not store or expose raw credentials, webhook secrets, respondent PII, raw
  response bodies, raw exports, raw provider payloads, prompts, manifests,
  package bytes, private keys, signatures, or unbounded response sets in
  observability.
- Do not silently publish forms, modify schemas, submit responses, export
  responses, create webhooks, or notify external systems without typed request,
  policy checks, idempotency, and approval where required.

## Ownership And Boundaries

- Pack id: `pack.office.forms.v1`.
- Family: `office`.
- Backing service owner: forms service provider.
- SDK surface: `sdk.packs.office.forms`.
- Command namespace: `forms.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridges, webhook
  verification bridges, artifact stores, decorators, and sanitized diagnostics
  through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `forms.inspect_provider` | Inspect provider/schema/response/webhook support | Returns sanitized schema, field, validation, submission, export, event, quota, and health metadata |
| `forms.create_form_request` | Create a form from metadata/schema handle | Requires idempotency key, write permission, publish policy, and audit |
| `forms.import_form_request` | Import form schema from artifact/schema handle | Requires artifact permission, schema validation, and audit |
| `forms.open_form` | Resolve form handle and version metadata | Requires form scope and bounded metadata |
| `forms.inspect_metadata` | Inspect title/owner/publish/response settings | Requires metadata permission and redaction |
| `forms.inspect_schema` | Inspect sections, fields, validation, logic, and publish settings | Requires schema permission, projection limits, and redaction |
| `forms.plan_schema_edit` | Plan field/section/validation/logic/publish edits | Validates schema operations, versions, provider support, approvals, and idempotency |
| `forms.schema_edit_request` | Execute a validated schema edit plan | Requires plan handle, idempotency key, version preconditions, and audit |
| `forms.create_response_session` | Create respondent/session envelope | Requires response scope, privacy policy, expiration, and replay handle |
| `forms.validate_response_draft` | Validate draft response values against schema | Requires schema version, redaction, and no submission side effect |
| `forms.submit_response_request` | Submit a validated response | Requires idempotency key, schema version, consent metadata, policy, and receipt |
| `forms.get_submission_receipt` | Resolve submission receipt metadata | Requires response scope and redaction |
| `forms.list_responses` | List response handles and bounded metadata | Requires response read permission, filters, paging, and retention |
| `forms.get_response` | Read one response projection | Requires response scope, PII redaction, and field-level policy |
| `forms.plan_response_export` | Plan response export artifact | Validates filters, sensitivity, retention, quotas, and approvals |
| `forms.response_export_request` | Execute response export | Returns bounded artifact handle and audit metadata |
| `forms.plan_event_subscription` | Plan webhook/watch subscription | Validates event types, endpoint/ref handle, signing policy, and approvals |
| `forms.event_subscription_request` | Create/renew/delete subscription | Requires plan handle, idempotency key, and sanitized diagnostics |
| `forms.inspect_events` | Inspect form schema/response event cursors | Requires event filters, redaction, paging, and retention |
| `forms.get_artifact_handle` | Resolve response/export/schema artifact metadata | Requires artifact permission, retention, and redaction |

Every command must define typed command DTOs, typed success results, typed
paged/partial/asynchronous results, typed denied/unavailable/unsupported/
conflict/stale-version/schema-mismatch/validation-failed/publish-denied/
submit-denied/export-denied/webhook-denied/webhook-signature-invalid/
response-redacted/quota/timeout/cancellation/approval-required/failure results,
redaction profile, idempotency semantics for side effects, and replay metadata.

## DTO Model

Core DTOs:

- `FormsScope`: provider scope handle, form handle, credential reference,
  network policy, artifact policy, permission state, privacy profile,
  rate-limit profile, and health.
- `FormsProviderCapability`: provider class, create/open/import support,
  schema-edit support, field-type support, validation support, conditional
  logic support, publish support, response validation/submission support,
  response read/export support, webhook/watch support, auth modes, rate limits,
  lifecycle, and health.
- `FormHandle`: form handle, provider scope, title handle, version hash,
  publish state, response collection state, owner/tenant handles, sensitivity
  class, and redaction class.
- `FormMetadata`: title/description handles, locale, owner handle, publish
  settings, response settings, quota class, retention, and redaction class.
- `FormSchema`: schema handle, form handle, version hash, section handles,
  field handles, validation rule handles, logic rule handles, capability hash,
  and redaction class.
- `FormSection`: section handle, schema handle, order class, title handle,
  description handle, visibility rule handles, and redaction class.
- `FormField`: field handle, section handle, field kind, title handle, help
  text handle, required state, option handles, validation handles, PII class,
  answer storage class, and redaction class.
- `FormFieldOption`: option handle, field handle, label handle, value handle,
  routing target handle, score/weight class, and redaction class.
- `FormValidationRule`: rule handle, field handle, rule kind, parameters
  handle, error-message handle, compatibility hash, and redaction class.
- `FormLogicRule`: rule handle, trigger field/option handles, action kind,
  target section/field handle, compatibility hash, and redaction class.
- `FormPublishSettings`: publish state, responder policy, collect-email state,
  auth requirement, response edit policy, confirmation message handle, and
  notification policy.
- `RespondentSession`: session handle, form handle, schema version hash,
  respondent handle, consent handle, expiration, idempotency scope, and
  redaction class.
- `FormResponseDraft`: draft handle, form handle, schema version hash, response
  values hash, validation diagnostics, consent state, and redaction class.
- `FormResponseValue`: value handle, field handle, answer kind, answer handle,
  validation state, PII class, and redaction class.
- `FormSubmissionReceipt`: receipt handle, form handle, response handle,
  submitted-at timestamp, idempotency key, provider receipt handle, and
  redaction class.
- `FormResponseExportPlan`: plan handle, form handle, response filters,
  field projection, output format, retention, redaction, required approvals,
  idempotency key, and validation diagnostics.
- `FormEventSubscriptionPlan`: plan handle, form handle, event types, endpoint
  reference handle, signing policy, renewal policy, required approvals,
  idempotency key, and validation diagnostics.
- `FormEvent`: event handle, form/response/schema handle, event kind, timestamp,
  actor/respondent handle, changed field classes, cursor, and redaction class.
- `FormArtifactHandle`: artifact handle, source form/response/export handle,
  artifact kind, content type, size class, checksum handle, retention,
  redaction class, and replay pointer.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `forms.provider.inspect`
- `forms.form.create`
- `forms.form.import`
- `forms.form.open`
- `forms.metadata.read`
- `forms.schema.read`
- `forms.schema.write`
- `forms.response.session`
- `forms.response.validate`
- `forms.response.submit`
- `forms.response.read`
- `forms.response.export`
- `forms.event.subscribe`
- `forms.event.read`
- `forms.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, form handle, response handle when applicable, actor
  or respondent handle when available, credential reference, network policy, and
  artifact policy.
- Schema edits, publish changes, submissions, exports, and event subscriptions
  require plan/request separation where side effects exist, idempotency keys,
  version preconditions, validation, consent metadata, and audit reason.
- PII fields, respondent identifiers, consent records, regulated answers,
  response exports, external webhooks, and public publish state may require
  approval.
- Raw response bodies, respondent PII, webhook payloads, exports, and artifacts
  require redaction and bounded output. Raw provider payloads must not enter
  observability.
- Remote operations require network permission, provider quota, rate limits,
  timeout, cancellation, webhook signature verification, and structured
  unavailable behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
field-type support, validation support, logic support, publish support,
response submission support, response read/export support, webhook/watch
support, permission scopes, policy templates, resource limits, approval rules,
provider capability hashes, health, compatibility, diagnostics, examples,
redaction profiles, and documentation links.

The developer guide at `docs/developer-packs/office/forms.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, form handles, metadata, schemas, sections, fields, options,
  validation rules, conditional logic, publish settings, respondent sessions,
  response drafts, response values, submission receipts, response exports,
  webhook/event subscriptions, event cursors, artifacts, provider capabilities,
  and unavailable states
- schema edit lifecycle, response validation/submission lifecycle, export
  lifecycle, webhook subscription lifecycle, schema/version conflicts, validation
  failures, PII redaction, consent metadata, approvals, quotas, provider
  replacement, trace/audit interpretation, and conformance tests

Examples must use synthetic forms, fields, respondents, responses, webhooks, and
artifacts. They must not include provider names, real credentials, webhook
secrets, private respondent data, raw responses, raw exports, or
workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `forms_pack_declared`
- `forms_pack_admission_validated`
- `forms_provider_inspected`
- `forms_form_created`
- `forms_form_imported`
- `forms_form_opened`
- `forms_metadata_inspected`
- `forms_schema_inspected`
- `forms_schema_edit_planned`
- `forms_schema_edit_requested`
- `forms_response_session_created`
- `forms_response_draft_validated`
- `forms_response_submitted`
- `forms_submission_receipt_resolved`
- `forms_responses_listed`
- `forms_response_resolved`
- `forms_response_export_planned`
- `forms_response_export_requested`
- `forms_event_subscription_planned`
- `forms_event_subscription_requested`
- `forms_events_inspected`
- `forms_artifact_handle_resolved`
- `forms_pack_policy_decision`
- `forms_pack_service_call_requested`
- `forms_pack_service_call_succeeded`
- `forms_pack_service_call_failed`
- `forms_pack_unavailable`
- `forms_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, form/schema
version hashes, command availability, provider health, policy template hash,
resource counters, bounded schema/field/response/export/subscription summaries,
event cursors, and sanitized replay pointers. Snapshots must exclude raw
credentials, webhook secrets, respondent PII, raw responses, raw exports, raw
provider payloads, prompts, manifests, package bytes, private keys, signatures,
and unbounded response sets.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, schema validators, response validators,
  export providers, webhook verifiers, redaction providers, and unavailable
  behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  credential redaction, webhook verification, idempotency, response redaction,
  and artifact retention wrap service calls.
- **Specification**: admission validates provider scope, command availability,
  permissions, schema compatibility, version preconditions, consent, provider
  state, quota, and compatibility.
- **Observer**: form schema events, response events, provider health, trace, and
  audit events are subscribable.
- **Memento**: form version hashes, schema handles, response sessions,
  submission receipts, export plans, event subscriptions, event cursors,
  snapshots, and replay pointers preserve recovery state.
- **Abstract Factory**: concrete form providers are created only by approved
  runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes a Google Forms/Typeform/Jotform wrapper. Mitigation:
  provider-neutral schema/field/response/event DTOs and Strategy adapters.
- Risk: respondent PII leaks. Mitigation: handles, PII classes, redaction,
  bounded summaries, artifact boundaries, and strict observability exclusions.
- Risk: duplicate submissions. Mitigation: idempotency keys, respondent session
  handles, schema version preconditions, submission receipts, and replay tests.
- Risk: provider response APIs differ or are unavailable. Mitigation: explicit
  provider capability DTO, unsupported diagnostics, and no fake compatibility.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call forms APIs directly.
