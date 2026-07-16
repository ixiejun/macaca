# Office Presentation Pack

`pack.office.presentation.v1` describes provider-neutral slide deck
capabilities. The pack is descriptor-only until a presentation provider is
installed through the runtime composition root.

## Manifest Declaration

Declare the pack as required only when deck access is mandatory for readiness.
Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.office.presentation.v1"]
```

## Permissions

Use the narrowest scope: `presentation.provider.inspect`,
`presentation.deck.create`, `presentation.deck.import`,
`presentation.deck.open`, `presentation.slide.read`,
`presentation.structure.read`, `presentation.asset.read`,
`presentation.notes.read`, `presentation.review.read`, `presentation.edit`,
`presentation.export`, `presentation.events.read`, and
`presentation.artifact.read`.

## Capability Model

Macaca models presentations as scoped deck handles, slide handles, structures,
themes and masters by reference, slide elements, media assets, notes, reviews,
edit plans, export plans, artifact handles, and collaboration events. Raw slide
media, private notes, review bodies, provider-native layout trees, credentials,
and provider payloads stay behind provider adapters.

## Platform Comparison

Google Slides API presentations, pages, page elements, masters, layouts,
comments, and batch updates map to deck, slide, element, review, and edit-plan
DTOs. Microsoft PowerPoint JavaScript and Graph concepts map to provider
adapter strategies. OpenXML PresentationML packages map to portable deck,
slide, asset, and export projections. Native layout engines and theme formats
remain provider-owned details.

## Commands

`presentation.inspect_provider`, `presentation.create_deck_request`,
`presentation.import_deck_request`, `presentation.open_deck`,
`presentation.list_slides`, `presentation.inspect_structure`,
`presentation.inspect_slide`, `presentation.inspect_assets`,
`presentation.inspect_notes`, `presentation.inspect_reviews`,
`presentation.plan_edit`, `presentation.edit_request`,
`presentation.plan_export`, `presentation.export_request`,
`presentation.inspect_events`, and `presentation.get_artifact_handle` are
descriptor-owned schema names.

## App-Facing Examples

- Inspect provider metadata before opening or creating a deck.
- Open a deck, list slides, and inspect structure using scoped handles.
- Read slide elements, assets, notes, and reviews through references and
  redaction profiles.
- Use `presentation.plan_edit` before `presentation.edit_request` for slide,
  layout, media, or notes changes.
- Export through artifact handles with an explicit redaction profile.
- Treat notes-redacted, asset-denied, export-denied, stale-version, and quota
  states as structured results.

## App-Facing Example Matrix

Generic examples cover provider inspection, deck create/import/open, slide
listing, structure inspection, slide inspection, asset inspection, notes/review
inspection, edit planning/request, export planning/request, event inspection,
and artifact handles with synthetic deck, slide, asset, review, event, and
artifact refs.

Diagnostic examples cover unavailable provider, missing deck permission, stale
version, slide-anchor stale, unsupported format, schema mismatch, export
denied, write approval, asset denied, notes redacted, provider quota, network
denied, and artifact denied. Diagnostics must not include provider names,
credentials, private notes, customer data, raw media, raw exports, or
workflow-specific conventions.

## Trace And Audit

Traces should record declaration, admission decision, command name, deck id,
slide id, provider class, capability hash, result status, export target, and
artifact id. They must not record raw media bytes, private notes, review bodies,
customer data, credentials, raw exports, or provider payloads.

## Provider Authors

Descriptors must report formats, max slides, slide and theme support, notes and
review support, asset handling, export formats, collaboration events, rate
limits, health, and snapshot metadata. Providers must return structured denied,
unavailable, unsupported, conflict, stale-version, schema-mismatch,
format-unsupported, asset-denied, notes-redacted, export-denied, write-denied,
quota, timeout, cancellation, approval-required, and failure results.

Conformance tests should cover descriptor completeness, deck and slide scope
validation, asset redaction, notes/review safety, edit validation, version
conflicts, export validation, artifact redaction, resource bounds, policy hooks,
trace and audit events, unavailable behavior, snapshot/replay, and redaction.
