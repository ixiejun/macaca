# Macaca Developer Packs

Macaca developer packs are provider-neutral capability contracts for
applications. A pack declaration grants discovery and admission semantics; actual
execution still requires a serviceized provider registered through the runtime
composition root.

## Foundation

- [Config](foundation/config.md): schema-backed configuration discovery,
  effective-value resolution, provenance, validation, watch/reload, redacted
  export, and unavailable source diagnostics.
- [Filesystem](foundation/filesystem.md): scoped roots, opaque handles, bounded
  reads/writes, directory listing, metadata, copy/move/delete, temp paths,
  watch streams, snapshots, restore dry-runs, and unavailable diagnostics.
- [Key-Value State](foundation/key-value-state.md): namespace-scoped typed
  values, revisions, compare-and-set, TTL, bounded scans, watch streams,
  snapshots, restore dry-runs, migration, compaction, and unavailable
  diagnostics.
- [Random](foundation/random.md): cryptographic random bytes, UUID v4, nonces,
  tokens, bounded integers, deterministic test streams, and entropy diagnostics.
- [Secrets Reference](foundation/secrets-reference.md): reference-only secret
  metadata, purpose binding, provider-only resolution handles, leases, rotation,
  audit access, and raw-secret-forbidden diagnostics.
- [Session State](foundation/session-state.md): session-scoped bounded values,
  revisions, checkpoints, restore dry-runs, compaction, redacted export, and
  recovery diagnostics.
- [Time](foundation/time.md): wall-clock and monotonic reads, timezone and
  calendar conversion, formatting, parsing, timers, deadlines, frozen test
  clocks, and clock-health diagnostics.

## Communication

- [Email](communication/email.md): compose, recipient validation, drafts, send,
  scheduled send, mailbox sync, attachments, delivery events, and unavailable
  diagnostics.
- [Messaging](communication/messaging.md): conversations, participants,
  send/reply/edit/delete, reactions, read receipts, attachments, event ingestion,
  and formatting capability diagnostics.
- [Notification](communication/notification.md): local, push, in-app,
  scheduled, interactive, subscription-backed, and inspectable notifications.
- [Inbox](communication/inbox.md): source registration, cursor sync, event
  ingestion, item/thread listing, body and attachment handles, triage, claims,
  and delegated summarization.
- [Calendar](communication/calendar.md): calendar sources, events, recurrence,
  invites, availability, reminders, conference handles, sync/watch, iCalendar,
  and conflict diagnostics.

## Knowledge

- [Search](knowledge/search.md): corpus registration, index schema, query AST,
  filters, facets, sorting, cursor pagination, suggestions, ranking
  explanations, ACL trimming, and unavailable diagnostics.
- [Retrieval](knowledge/retrieval.md): collections, namespaces, records,
  chunks, vector-space compatibility, metadata filters, hybrid retrieval,
  rerank, context expansion, evidence packaging, and refresh diagnostics.
- [Document Parsing](knowledge/document-parsing.md): document handles,
  validation, sync/async parsing, OCR, layout, tables, forms, metadata,
  canonical conversion, chunking, geometry, confidence, and parser diagnostics.
- [Citations](knowledge/citations.md): citation metadata, identifiers,
  contributors, source anchors, selectors, evidence, bibliography styles,
  verification, import/export, and source-anchor diagnostics.
- [Graph](knowledge/graph.md): property graph and RDF-style stores, schemas,
  nodes, edges, triples, queries, traversal, paths, import/export, merge,
  provenance, and provider replacement diagnostics.
- [Summarization](knowledge/summarization.md): summary planning, extractive and
  abstractive modes, cited summaries, multi-source synthesis, conversation
  summaries, context compression, comparison, evaluation, and evidence links.

## Office

- [Document](office/document.md): document handles, structures, ranges, styles,
  comments, revisions, edit plans, export plans, artifacts, collaboration
  events, and unavailable diagnostics.
- [Spreadsheet](office/spreadsheet.md): workbook and worksheet handles, bounded
  ranges, formulas, named ranges, tables, charts, pivots, update plans,
  recalculation, exports, and unavailable diagnostics.
- [Presentation](office/presentation.md): deck and slide handles, structures,
  slide elements, media assets, notes, reviews, edit plans, exports,
  artifacts, and unavailable diagnostics.
- [PDF](office/pdf.md): PDF handles, metadata, pages, render and extraction
  plans, forms, annotations, embedded files, signatures, redaction, merge/split,
  exports, and unavailable diagnostics.
- [Forms](office/forms.md): form handles, metadata, schemas, fields,
  validation, conditional logic, respondent sessions, responses, exports,
  event subscriptions, and unavailable diagnostics.

## Media

- [Image](media/image.md): image handles, metadata, geometry, color profiles,
  frames, transforms, composites, annotations, redactions, generation and edit
  plans, safety reports, exports, artifacts, and unavailable diagnostics.
- [Audio](media/audio.md): audio handles, metadata, waveform summaries,
  segments, filters, mix graphs, voice capabilities, synthesis plans, exports,
  artifacts, and unavailable diagnostics.
- [Video](media/video.md): video handles, metadata, tracks, frames, timeline
  ranges, thumbnails, transcodes, segments, renders, subtitles, packages, jobs,
  exports, artifacts, and unavailable diagnostics.
- [Transcription](media/transcription.md): source handles, media metadata,
  batch and streaming plans, chunk handles, transcript projections, diarization,
  redaction, subtitles, translation handoff, jobs, artifacts, and unavailable
  diagnostics.
- [Rendering](media/rendering.md): source handles, template metadata, scene
  summaries, viewports, surfaces, assets, fonts, render/frame/animation/preview
  plans, jobs, exports, artifacts, and unavailable diagnostics.

## Finance

- [Market Data](finance/market-data.md): instruments, venues, sessions,
  quotes, trades, bars, snapshots, corporate actions, freshness, attribution,
  cursors, artifacts, and unavailable diagnostics.
- [Stock](finance/stock.md): equities, company profiles, listings, statements,
  facts, metrics, corporate events, screens, universes, quote handoff,
  freshness, attribution, artifacts, and unavailable diagnostics.
- [Crypto](finance/crypto.md): crypto assets, token references, chains,
  venues, market pairs, quotes, trades, bars, snapshots, supply metrics, public
  address references, oracle feeds, freshness, attribution, and unavailable
  diagnostics.
- [Accounting](finance/accounting.md): entities, ledger books, periods, chart
  of accounts, account mutations, journals, ledger entries, reconciliation,
  reports, audit exports, artifacts, redaction, and unavailable diagnostics.
- [Portfolio](finance/portfolio.md): consent-scoped accounts, instruments,
  positions, lots, cash balances, transactions, valuation, allocation,
  exposure, performance, risk, scenarios, rebalance intents, reports, and
  unavailable diagnostics.
- [Invoice](finance/invoice.md): schema discovery, parties, items, invoice
  planning, draft creation, issue, delivery, payment-status sync, reminders,
  voiding, exports, artifacts, recipient policy, and unavailable diagnostics.

## Commerce

- [Catalog](commerce/catalog.md): products, variants, attributes, options,
  modifiers, price books, price contexts, availability snapshots, taxonomy,
  publication scopes, portable search, mutation plans, exports, artifacts, and
  unavailable diagnostics.
- [Cart](commerce/cart.md): cart lifecycle, buyer context, lines, discounts,
  estimates, validation issues, stale diagnostics, handoff intents, exports,
  artifacts, and unavailable diagnostics.
- [Order](commerce/order.md): order records, lifecycle states, status sync,
  fulfillment-intent references, cancellation, return references, audit exports,
  artifacts, and unavailable diagnostics.
- [Payment Intent](commerce/payment-intent.md): intent planning, tokenized
  payment-method references, confirmation, action inspection, capture,
  cancellation, status sync, idempotency, event references, audit exports, and
  unavailable diagnostics.
- [Receipt](commerce/receipt.md): receipt evidence, issue/reissue,
  read/search, source sync, verification, delivery, correction references,
  event references, artifacts, and unavailable diagnostics.
- [Entitlement](commerce/entitlement.md): subject/resource grants, checks,
  batch checks, source sync, state transitions, seats, usage metering, event
  references, proof exports, artifacts, and unavailable diagnostics.

## Identity

- [Account](identity/account.md): account records, minimized identifiers,
  lifecycle states, linked identities, recovery references, audit references,
  artifacts, and unavailable diagnostics.
- [Profile](identity/profile.md): profile fields, schema descriptors, privacy
  classes, profile-owned preferences, avatar references, exports, artifacts,
  and unavailable diagnostics.
- [Auth Handoff](identity/auth-handoff.md): handoff planning, callback
  verification, token/assertion references, subject evidence, session-binding
  evidence, replay protection, artifacts, and unavailable diagnostics.
- [Organization](identity/organization.md): organization records,
  memberships, invitations, role bindings, directory references, audit exports,
  artifacts, and unavailable diagnostics.
- [Tenant](identity/tenant.md): tenant records, isolation policy references,
  quota envelopes, usage snapshots, residency hints, config references,
  artifacts, and unavailable diagnostics.

## Developer

- [Code](developer/code.md): workspaces, documents, syntax summaries, symbols,
  diagnostics, code actions, edit plans, patches, diffs, impact reports, scan
  findings, test suggestions, and unavailable diagnostics.
- [Repository](developer/repository.md): repositories, remotes, refs, branches,
  tags, commits, status entries, diffs, mutation plans, sync plans, and
  unavailable diagnostics.
- [CI](developer/ci.md): projects, pipelines, runs, jobs, steps, statuses,
  trigger plans, mutation plans, logs, artifacts, test reports, environments,
  and unavailable diagnostics.
- [Issue Tracker](developer/issue-tracker.md): projects, field schemas,
  issues, comments, labels, milestones, workflow states, transition plans,
  update plans, search, relations, attachments, timelines, and unavailable
  diagnostics.
- [Terminal](developer/terminal.md): scopes, process specs, environment
  policies, workdir scopes, PTY profiles, spawn plans, sessions, streams,
  stdin frames, signals, exit status, resource usage, snapshots, and
  unavailable diagnostics.
- [Browser Automation](developer/browser-automation.md): contexts, pages,
  frames, locators, navigation plans, action plans, evaluation plans, waits,
  artifacts, network/console/dialog events, storage handles, snapshots, and
  unavailable diagnostics.
- [Design Tools](developer/design-tools.md): workspaces, files, pages, nodes,
  components, styles, tokens, token-sync plans, export plans, artifacts, change
  sets, component mappings, reviews, and unavailable diagnostics.

## Location

- [Maps](location/maps.md): style references, tile matrices, tile references,
  viewports, annotations, overlays, static render plans, attribution bundles,
  cache status, artifacts, and unavailable diagnostics.
- [Geocode](location/geocode.md): forward and reverse geocoding, address
  normalization, candidates, geometry, precision classes, confidence,
  retention, batch jobs, artifacts, and unavailable diagnostics.
- [Route](location/route.md): waypoints, travel profiles, constraints, route
  plans, legs, steps, maneuvers, geometry references, metrics, matrices,
  optimization jobs, artifacts, and unavailable diagnostics.
- [Place Search](location/place-search.md): text search, nearby search,
  autocomplete, suggestion resolution, place details, categories, field masks,
  attribution, sessions, and unavailable diagnostics.
- [Timezone](location/timezone.md): coordinate lookup, zone resolution,
  offsets, transitions, instant conversion, local-time gap/fold handling,
  display names, database metadata, mappings, and unavailable diagnostics.

## Device

- [Sensors](device/sensors.md): sensor descriptors, types, readings,
  coordinate frames, accuracy, calibration, bounded batches, stream leases,
  host status, and unavailable diagnostics.
- [Camera](device/camera.md): authorization, camera descriptors,
  constraints, sessions, preview leases, frame references, media references,
  controls, privacy indicators, and unavailable diagnostics.
- [Local Files](device/local-files.md): picker-mediated handles, grants,
  metadata, filters, chunks, transfers, directory entries, write plans, host
  status, and unavailable diagnostics.
- [Notifications](device/notifications.md): authorization, channels,
  categories, actions, content references, triggers, delivery policy, records,
  interactions, badge support, and unavailable diagnostics.
- [Foreground/Background Host](device/foreground-background-host.md): host
  lifecycle states, foreground sessions, background leases, events, policies,
  throttling, suspension, revocation, and unavailable diagnostics.

## AI

- [LLM](ai/llm.md): chat, completion, provider-neutral routing metadata, token
  estimation, budget inspection, cancellation, tool-call metadata, streaming
  frames, and unavailable diagnostics.
- [Embedding](ai/embedding.md): text/image embedding, batch embedding, vector
  schema inspection, usage estimation, stable item mapping, and unavailable
  diagnostics.
- [Rerank](ai/rerank.md): query/candidate references, deterministic ranking,
  batch reranking, score explanations, evaluation metadata, and unavailable
  diagnostics.
- [Vision](ai/vision.md): image/video analysis, OCR, object detection, visual
  moderation, normalized regions, visual evidence refs, jobs, and unavailable
  diagnostics.
- [Speech](ai/speech.md): speech-to-text, text-to-speech, voice catalogs,
  speech translation, timing alignment, stream frames, and unavailable
  diagnostics.
- [Model Evaluation](ai/model-evaluation.md): eval definitions, dataset/sample
  refs, graders, runs, metrics, comparisons, redacted reports, and unavailable
  diagnostics.

## Workflow

- [Task](workflow/task.md): task specs, queues, dependencies, leases, attempts,
  progress, checkpoints, artifact references, terminal states, and unavailable
  diagnostics.
- [Schedule](workflow/schedule.md): recurrences, timezone policies, misfire and
  overlap policies, trigger records, bounded backfills, snapshots, and
  unavailable diagnostics.
- [Approval](workflow/approval.md): approval requests, assignments, decisions,
  evidence bundles, escalation, decision gates, and unavailable diagnostics.
- [Delegation](workflow/delegation.md): delegation requests, candidate pools,
  claims, capacity snapshots, leases, handoffs, result references, and
  unavailable diagnostics.
- [Review](workflow/review.md): review requests, rounds, findings, fix
  requests, rereviews, outcomes, closure gates, and unavailable diagnostics.
- [Recovery](workflow/recovery.md): failure records, recovery points, retry
  policies, repair plans, compensation references, resume plans, replay exports,
  and unavailable diagnostics.
