# Change: Add Office Forms Pack

## Why

Developers need `pack.office.forms.v1` as an industrial forms capability for
form provider inspection, form creation/import/opening, schema inspection,
question and section modeling, validation rule modeling, draft/publish
settings, respondent session handling, response validation, response
submission, response retrieval/export, webhook/event subscription, audit, and
replay diagnostics. It must not be a thin wrapper around Google Forms,
Typeform, Jotform, Microsoft Forms, a survey builder, or one form-rendering
library.

Forms can collect regulated identity, health, employment, finance, school,
customer, consent, and survey data. Creating or publishing forms can expose
collection endpoints; submitting responses can perform external side effects;
exporting responses can leak personal data at scale. Macaca must therefore
expose form operations only through provider-neutral typed service commands
with permission, policy, entitlement, resource, approval, validation,
idempotency, webhook verification, redaction, artifact retention, trace, audit,
health, snapshot, replay, and structured unavailable behavior.

## Research And Supplier/API Baseline

Official and supplier references considered for this pack:

- Google Forms API exposes form create/get, `forms.batchUpdate` for structural
  updates, publish settings, response get/list, and watches for form schema and
  response events. References:
  https://developers.google.com/workspace/forms/api/reference/rest and
  https://developers.google.com/workspace/forms/api/reference/rest/v1/forms/batchUpdate
- Typeform developer platform exposes Create API for form/theme/image
  management, Responses API for response retrieval, and Webhooks API for
  real-time submission delivery. References:
  https://www.typeform.com/developers/,
  https://www.typeform.com/developers/responses/, and
  https://www.typeform.com/developers/webhooks/
- Jotform API exposes account/form/submission access and webhooks for
  submission notifications. Reference: https://api.jotform.com/docs/
- Microsoft Graph has broad OData, paging, auth, and enterprise integration
  patterns, while Microsoft Forms response APIs are not generally available in
  Microsoft Graph at this time. This gap must be represented as provider
  capability metadata and structured unavailable/unsupported diagnostics rather
  than hidden fallback. Reference: https://learn.microsoft.com/en-us/graph/use-the-api

Macaca maps these supplier concepts into provider-neutral form scope, provider
capability, form handle, form schema, section/page, field/question, choice
option, validation rule, conditional logic, publish settings, respondent
session, response draft, response value, submission receipt, response export
plan, webhook/event subscription, artifact handle, provider capability,
version/freshness metadata, and diagnostics DTOs. Concrete Google Forms,
Typeform, Jotform, Microsoft Forms, renderers, webhooks, storage, spreadsheet,
notification, and export providers stay behind replaceable providers.

## What Changes

- Add provider-neutral `pack.office.forms.v1` under the `office` family.
- Define command namespace `forms.*` for:
  - provider capability inspection
  - form creation/import/opening and metadata inspection
  - schema, section, field/question, validation, logic, and publish-state
    inspection
  - schema edit planning and edit requests
  - respondent session creation and response draft validation
  - response submission and receipt retrieval
  - response listing, retrieval, export planning, and export requests
  - webhook/event subscription planning and subscription requests
  - event inspection, artifact handle resolution, snapshots, and replay
- Define DTOs for forms scope, provider capability, form handle, form metadata,
  form schema, section, field, field option, validation rule, conditional logic,
  publish settings, respondent session, response draft, response value,
  submission receipt, response export plan, webhook subscription plan, event
  cursor, artifact handle, and diagnostics.
- Define permission scopes, policy defaults, form/response/artifact scopes,
  PII/consent redaction, webhook signature behavior, idempotent submission,
  approval rules, resource/entitlement behavior, SDK discovery, developer
  documentation, trace/audit events, snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/office/forms.md` before implementation completion.

## Impact

- Affected specs: `pack-office-forms`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, forms service
  provider or unavailable provider, runtime-host provider adapters,
  webhook/event verification support, response/artifact/redaction support,
  trace/audit schemas, replay tests, dependency-boundary gates, and developer
  documentation.
- Non-goals: no concrete Google Forms/Typeform/Jotform/Microsoft Forms/
  spreadsheet/export/notification/webhook provider implementation in this
  proposal; no survey, HR, medical, legal, school, CRM, quiz, order, lead-gen,
  consent, or workflow-specific business logic; no provider-name, form-name,
  field-name, webhook-name, respondent-name, or workflow-name routing in OS
  layers; no raw credentials, webhook secrets, respondent PII, raw responses,
  raw exports, raw provider payloads, prompts, manifests, package bytes, private
  keys, signatures, or unbounded response sets in observability; no SDK/shell/
  kernel provider construction; no fake success when provider, schema support,
  response support, webhook support, permission, entitlement, approval, resource,
  validation, or host support is absent.
