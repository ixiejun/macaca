# Change: Add Developer Design Tools Pack

## Why

Developers need `pack.developer.design.tools.v1` as an industrial design-tool
capability for design file discovery, page/canvas inspection, node tree
inspection, component/library metadata, style and token synchronization, asset
export, design-to-code mapping, change-set planning, design mutations where
policy allows, comments/review metadata, and replayable artifact diagnostics. It
must not be a thin wrapper around Figma, Adobe UXP/Photoshop, Penpot, Sketch, or
one design-system workflow.

Design tools contain product plans, unreleased UI, brand assets, customer data,
comments, screenshots, private tokens, and shared libraries. Writing design
changes can modify source-of-truth assets and notify collaborators. Macaca must
therefore expose design-tool behavior only through provider-neutral typed
service commands with permission, policy, entitlement, resource, approval,
redaction, trace, audit, health, snapshot, replay, and structured unavailable
behavior.

## Research And Supplier/API Baseline

Official references considered for this pack:

- Figma REST API exposes files, nodes, images, comments, components, component
  sets, styles, projects, teams, and rate-limited authenticated access.
  References: https://developers.figma.com/docs/rest-api/ and
  https://developers.figma.com/docs/rest-api/file-endpoints/
- Figma Plugin API exposes document node trees, components/instances, variables,
  styles, export settings, and in-file mutations. References:
  https://developers.figma.com/docs/plugins/api/ComponentNode/ and
  https://developers.figma.com/docs/plugins/working-with-variables/
- Adobe Photoshop UXP exposes documents, layers, actions, export/output, and
  plugin/scripting access to design/image documents. Reference:
  https://developer.adobe.com/photoshop/uxp/2022/ps-reference/
- Penpot design tokens align with W3C DTCG token structures for cross-tool token
  exchange. Reference:
  https://help.penpot.app/user-guide/design-systems/design-tokens/

Macaca maps these supplier concepts into provider-neutral design workspace,
design file, page/canvas, node, component, component set, instance, style,
token, library, export artifact, comment/review event, change set, operation
plan, and provider capability DTOs. Concrete design tools, plugin runtimes,
REST APIs, OAuth flows, desktop automation bridges, and provider-specific node
models stay behind replaceable providers.

## What Changes

- Add provider-neutral `pack.developer.design.tools.v1` under the `developer`
  family.
- Define command namespace `design_tools.*` for:
  - provider/workspace/library capability inspection
  - design file/project discovery
  - page/canvas and node tree inspection
  - component, component set, instance, style, and variable/token inspection
  - token import/export/sync planning
  - asset export planning and export requests
  - design-to-code mapping and component binding
  - design change planning and write requests
  - comment/review metadata inspection where supported
  - artifact handle retrieval, snapshots, and replay diagnostics
- Define DTOs for design scope, provider capability, design workspace, design
  file, page/canvas, node, component, style, token, variable collection, library,
  export plan, artifact handle, change set, write plan, component mapping,
  comment/review event, version/freshness metadata, and diagnostics.
- Define permission scopes, policy defaults, library/file/node scopes,
  version-precondition behavior, token schema compatibility, artifact redaction,
  resource/entitlement behavior, approval rules, SDK discovery, developer
  documentation, trace/audit events, snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/developer/design-tools.md` before implementation
  completion.

## Impact

- Affected specs: `pack-developer-design-tools`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, design-tool service
  provider or unavailable provider, runtime-host provider adapters,
  artifact/token/redaction support, trace/audit schemas, replay tests,
  dependency-boundary gates, and developer documentation.
- Non-goals: no concrete Figma/Adobe/Penpot/Sketch provider implementation in
  this proposal; no app-specific design workflow, brand system, or UI generation
  logic; no provider-name, tool-name, file-name, component-name, token-name, or
  workflow-name routing in OS layers; no raw access tokens, private comments,
  full design files, raw image assets, unpublished designs, provider payloads,
  prompts, manifests, or unbounded node trees in observability; no
  SDK/shell/kernel provider construction; no fake success when provider,
  workspace scope, entitlement, permission, version, approval, resource, or host
  support is absent.
