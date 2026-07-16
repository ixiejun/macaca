# Office Forms Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.office.forms.v1`. Forms support must expose schema, field, page, logic,
publishing, response, webhook/watch, validation, export, and provider-capability
operations through serviceized commands, not provider-native survey APIs or
application-specific school, HR, medical, CRM, quiz, order, or lead workflows.

## Source Baseline

- Google Forms API:
  <https://developers.google.com/workspace/forms/api/reference/rest>
  and Google Forms API overview:
  <https://developers.googleblog.com/introducing-the-google-forms-api/>
- Typeform Create, Responses, and Webhooks APIs:
  <https://www.typeform.com/developers/create/>
  <https://www.typeform.com/developers/responses/>
  <https://www.typeform.com/developers/webhooks/>
- Jotform API and webhooks:
  <https://api.jotform.com/docs/>
  and <https://www.jotform.com/features/forms-api-webhooks/>
- Microsoft Graph/Microsoft Forms official gap is modeled as provider
  unsupported/unavailable unless Microsoft ships a supported Forms response API.
- HTML form and JSON Schema concepts are neutral schema inspiration only, not
  an OS-owned renderer model.

## Supplier API Notes

- Google Forms contributes form create/get, `forms.batchUpdate`, publish
  settings, response get/list, watches, and schema/response events. Macaca
  should model form schema, mutation batch, publish state, response cursors, and
  watch handles.
- Typeform contributes form creation/update, themes/images, logic jumps,
  response retrieval, webhook delivery, event signing, and response JSON.
  Macaca should map these into schema, presentation metadata, logic graph,
  response envelopes, webhook signatures, and provider capability.
- Jotform contributes account/form/submission access, mostly read-oriented v1
  API behavior, webhook notifications, and form/submission retrieval. Macaca
  should model provider capability degradation and webhook behavior explicitly.
- Microsoft Forms/Graph gaps must become structured unsupported/unavailable
  diagnostics, not hidden compatibility assumptions or unofficial endpoint use.
- HTML/JSON Schema contribute field validation, required/optional constraints,
  primitive types, and renderer-neutral schema vocabulary, but Macaca should not
  hardcode a browser renderer as the pack contract.

## Macaca-Owned Abstractions

`pack.office.forms.v1` should define `FormHandle`, `FormSchema`,
`FormPage`, `FormField`, `FormChoice`, `FormValidationRule`,
`FormLogicRule`, `FormThemeHint`, `FormPublishState`, `FormResponse`,
`FormResponseCursor`, `FormWebhook`, `FormWatch`, `FormExportPlan`, and
`FormProviderCapability`.

The DTOs must carry ownership, schema version, field typing, validation,
conditional logic, publish state, response redaction, webhook signature
metadata, watch cursors, export limits, provider capability hashes, and replay
pointers. Raw provider payloads, raw responses beyond policy, credentials,
webhook secrets, private answer values, and unbounded exports are rejected.

## Explicit Non-Goals

- Do not implement concrete Google Forms, Typeform, Jotform, Microsoft Forms,
  renderer, webhook provider, survey builder, or storage provider adapters in
  this research phase.
- Do not define survey, HR, medical, legal, school, CRM, quiz, order, payment,
  or lead-generation workflows in OS layers.
- Do not use unofficial Microsoft Forms APIs or expose provider-native schemas
  as Macaca application contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, notification/watch patterns, and file/export handles provide
  reusable substrate.
- Current evidence does not prove forms DTOs, providers, SDK helpers, WASM ABI
  metadata, webhook verification tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
