# Child OpenSpec Proposal Index

This umbrella change intentionally does not implement every industrial sub-pack
directly. Each row below is a required child OpenSpec proposal. A child proposal
must be detailed enough that completing its tasks yields an industrial-grade
usable pack, or explicitly records why that sub-pack is preview/unavailable.

Every child proposal must include:

- service ownership and provider boundary;
- typed command/result contracts;
- permission scopes, policy, resource, entitlement, and approval behavior;
- SDK discovery metadata and examples;
- trace, audit, health, snapshot, replay, and unavailable diagnostics;
- tests and executable gates proving canonical service-path execution;
- no application-specific, provider-specific, or OS-layer business branches.

| Family task | Sub-pack | Required child proposal |
| --- | --- | --- |
| foundation | filesystem | `add-pack-foundation-filesystem` |
| foundation | key-value state | `add-pack-foundation-key-value-state` |
| foundation | time | `add-pack-foundation-time` |
| foundation | random | `add-pack-foundation-random` |
| foundation | config | `add-pack-foundation-config` |
| foundation | secrets reference | `add-pack-foundation-secrets-reference` |
| foundation | session state | `add-pack-foundation-session-state` |
| communication | email | `add-pack-communication-email` |
| communication | messaging | `add-pack-communication-messaging` |
| communication | notification | `add-pack-communication-notification` |
| communication | inbox | `add-pack-communication-inbox` |
| communication | calendar | `add-pack-communication-calendar` |
| knowledge | search | `add-pack-knowledge-search` |
| knowledge | retrieval | `add-pack-knowledge-retrieval` |
| knowledge | document parsing | `add-pack-knowledge-document-parsing` |
| knowledge | citations | `add-pack-knowledge-citations` |
| knowledge | graph | `add-pack-knowledge-graph` |
| knowledge | summarization | `add-pack-knowledge-summarization` |
| developer | code | `add-pack-developer-code` |
| developer | repository | `add-pack-developer-repository` |
| developer | CI | `add-pack-developer-ci` |
| developer | issue tracker | `add-pack-developer-issue-tracker` |
| developer | terminal | `add-pack-developer-terminal` |
| developer | browser automation | `add-pack-developer-browser-automation` |
| developer | design tools | `add-pack-developer-design-tools` |
| office | document | `add-pack-office-document` |
| office | spreadsheet | `add-pack-office-spreadsheet` |
| office | presentation | `add-pack-office-presentation` |
| office | PDF | `add-pack-office-pdf` |
| office | forms | `add-pack-office-forms` |
| media | image | `add-pack-media-image` |
| media | audio | `add-pack-media-audio` |
| media | video | `add-pack-media-video` |
| media | transcription | `add-pack-media-transcription` |
| media | rendering | `add-pack-media-rendering` |
| finance | market data | `add-pack-finance-market-data` |
| finance | stock | `add-pack-finance-stock` |
| finance | crypto | `add-pack-finance-crypto` |
| finance | accounting | `add-pack-finance-accounting` |
| finance | portfolio | `add-pack-finance-portfolio` |
| finance | invoice | `add-pack-finance-invoice` |
| commerce | catalog | `add-pack-commerce-catalog` |
| commerce | cart | `add-pack-commerce-cart` |
| commerce | order | `add-pack-commerce-order` |
| commerce | payment intent | `add-pack-commerce-payment-intent` |
| commerce | receipt | `add-pack-commerce-receipt` |
| commerce | entitlement | `add-pack-commerce-entitlement` |
| identity | account | `add-pack-identity-account` |
| identity | profile | `add-pack-identity-profile` |
| identity | auth handoff | `add-pack-identity-auth-handoff` |
| identity | organization | `add-pack-identity-organization` |
| identity | tenant | `add-pack-identity-tenant` |
| location | maps | `add-pack-location-maps` |
| location | geocode | `add-pack-location-geocode` |
| location | route | `add-pack-location-route` |
| location | place search | `add-pack-location-place-search` |
| location | timezone | `add-pack-location-timezone` |
| device | sensors | `add-pack-device-sensors` |
| device | camera | `add-pack-device-camera` |
| device | local files | `add-pack-device-local-files` |
| device | notifications | `add-pack-device-notifications` |
| device | foreground/background host capabilities | `add-pack-device-foreground-background-host` |
| ai | LLM | `add-pack-ai-llm` |
| ai | embedding | `add-pack-ai-embedding` |
| ai | rerank | `add-pack-ai-rerank` |
| ai | vision | `add-pack-ai-vision` |
| ai | speech | `add-pack-ai-speech` |
| ai | model evaluation | `add-pack-ai-model-evaluation` |
| workflow | task | `add-pack-workflow-task` |
| workflow | schedule | `add-pack-workflow-schedule` |
| workflow | approval | `add-pack-workflow-approval` |
| workflow | delegation | `add-pack-workflow-delegation` |
| workflow | review | `add-pack-workflow-review` |
| workflow | recovery | `add-pack-workflow-recovery` |
