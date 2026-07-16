# Office Forms Pack

`pack.office.forms.v1` describes provider-neutral form schema, response, export,
and event capabilities. The pack is descriptor-only until a forms provider is
installed through the runtime composition root.

## Manifest Declaration

Declare the pack as required only when form access is mandatory for readiness.
Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.office.forms.v1"]
```

## Permissions

Use the narrowest scope: `forms.provider.inspect`, `forms.form.create`,
`forms.form.import`, `forms.form.open`, `forms.metadata.read`,
`forms.schema.read`, `forms.schema.write`, `forms.response.session`,
`forms.response.validate`, `forms.response.submit`, `forms.response.read`,
`forms.response.export`, `forms.event.subscribe`, `forms.event.read`, and
`forms.artifact.read`.

## Capability Model

Macaca models forms as scopes, form handles, metadata, schemas, sections,
fields, options, validation rules, conditional logic rules, publish settings,
respondent sessions, response drafts, response values by reference, submission
receipts, response export plans, event subscription plans, event cursors, and
artifact handles. Raw responses, respondent PII, webhook secrets, provider
question models, credentials, and provider payloads stay behind adapters.

## Platform Comparison

Google Forms create/get, batch update, publish settings, responses, and watches
map to form, schema, publish, response, and event DTOs. Typeform Create,
Responses, and Webhooks APIs map to schema, response export, and event
subscription DTOs. Jotform form/submission APIs map to provider strategies.
Microsoft Forms and Graph capability gaps are represented as structured
unsupported or unavailable states. HTML and JSON Schema concepts inform neutral
field and validation DTOs but do not define OS semantics.

## Commands

`forms.inspect_provider`, `forms.create_form_request`,
`forms.import_form_request`, `forms.open_form`, `forms.inspect_metadata`,
`forms.inspect_schema`, `forms.plan_schema_edit`,
`forms.schema_edit_request`, `forms.create_response_session`,
`forms.validate_response_draft`, `forms.submit_response_request`,
`forms.get_submission_receipt`, `forms.list_responses`, `forms.get_response`,
`forms.plan_response_export`, `forms.response_export_request`,
`forms.plan_event_subscription`, `forms.event_subscription_request`,
`forms.inspect_events`, and `forms.get_artifact_handle` are descriptor-owned
schema names.

## App-Facing Examples

- Inspect provider metadata before creating, importing, or opening a form.
- Inspect metadata and schema using scoped form handles.
- Use schema edit plans before changing fields, validation, logic, or publish
  state.
- Create respondent sessions and validate drafts before submission.
- Use idempotency keys for submissions and receipts for audit.
- List, get, and export responses through references and redaction profiles.
- Plan event subscriptions before creating webhook/watch behavior.
- Treat validation-failed, publish-denied, submit-denied, response-redacted,
  webhook-denied, webhook-signature-invalid, and quota states as structured
  results.

## App-Facing Example Matrix

Generic examples cover provider inspection, create/import/open, metadata
inspection, schema inspection, schema edit planning/request, response session
creation, draft validation, response submission, receipt retrieval, response
listing/retrieval, response export planning/request, event subscription
planning/request, event inspection, and artifact handles with synthetic form,
schema, response, receipt, event, and artifact refs.

Diagnostic examples cover unavailable provider, missing form permission, stale
schema version, schema mismatch, validation failed, publish denied, submit
denied, duplicate idempotency, response redacted, export approval, webhook
denied, webhook signature invalid, provider quota, network denied, and artifact
denied. Diagnostics must not include provider names, credentials, webhook
secrets, private respondent data, raw responses, raw exports, or
workflow-specific conventions.

## Trace And Audit

Traces should record declaration, admission decision, command name, form id,
schema version hash, field count, provider class, capability hash, result
status, response hash, event cursor hash, and artifact id. They must not record
webhook secrets, respondent PII, raw answers, raw exports, credentials, or
provider payloads.

## Provider Authors

Descriptors must report supported field types, max fields, validation support,
conditional logic support, publish behavior, response session behavior,
submission support, response read/export support, webhook/watch support, signing
policy, health, and snapshot metadata. Providers must return structured denied,
unavailable, unsupported, conflict, stale-version, schema-mismatch,
validation-failed, publish-denied, submit-denied, export-denied,
webhook-denied, webhook-signature-invalid, response-redacted, quota, timeout,
cancellation, approval-required, and failure results.

Conformance tests should cover descriptor completeness, form/schema/field and
response scope validation, schema compatibility, validation logic, conditional
logic, publish state, respondent sessions, idempotent submissions, PII redaction,
response export validation, webhook signature validation, event cursors,
artifact redaction, resource bounds, policy hooks, trace and audit events,
unavailable behavior, snapshot/replay, and redaction.
