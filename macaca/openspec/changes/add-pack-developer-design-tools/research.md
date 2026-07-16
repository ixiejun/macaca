# Developer Design Tools Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.developer.design_tools.v1`. Design tool support must expose files, nodes,
images, comments, components, styles, variables, tokens, layers, actions,
exports, and diagnostics through serviceized commands. It must not hardcode
Figma, Adobe, Penpot, Sketch, OAuth, design-to-code, brand, or provider-native
plugin workflows into OS-layer semantics.

## Source Baseline

- Figma REST API files, nodes, images, components, styles, and webhooks:
  <https://developers.figma.com/docs/rest-api/>
  <https://developers.figma.com/docs/rest-api/file-endpoints/>
  <https://developers.figma.com/docs/rest-api/component-types/>
- Figma Plugin API and variables:
  <https://developers.figma.com/docs/plugins/>
  and <https://developers.figma.com/docs/plugins/working-with-variables/>
- Adobe Photoshop UXP Photoshop API and manifest:
  <https://developer.adobe.com/photoshop/uxp/2022/ps-reference/>
  and
  <https://developer.adobe.com/photoshop/uxp/2022/guides/uxp-guide/uxp-misc/manifest-v4/photoshop-manifest/>
- Penpot design tokens and W3C DTCG:
  <https://penpot.app/blog/design-tokens-with-penpot/>
  and <https://www.designtokens.org/>

## Supplier API Notes

- Figma REST contributes files, nodes, images, comments, published components,
  component sets, styles, projects, teams, authentication, webhooks, and rate
  limits. Macaca should model design file and node handles, component/style
  metadata, image export artifacts, and webhook/watch behavior.
- Figma Plugin API contributes in-file read/write access to document nodes,
  components, instances, variables, styles, export settings, and mutations.
  Macaca should model mutation capability and plugin-host boundaries without
  exposing the Figma global object.
- Photoshop UXP contributes documents, layers, actions, output/export, plugin
  manifests, scripting, host permissions, and desktop host lifecycle. Macaca
  should model local host capability, action plans, and unavailable/permission
  diagnostics.
- Penpot and W3C DTCG contribute token sets, themes, aliases, standardized token
  JSON, modern color spaces, and interoperability. Macaca should model design
  tokens as provider-neutral assets rather than one tool's token schema.

## Macaca-Owned Abstractions

`pack.developer.design_tools.v1` should define `DesignFile`,
`DesignNode`, `DesignImageExport`, `DesignComment`, `DesignComponent`,
`DesignComponentSet`, `DesignStyle`, `DesignVariable`, `DesignToken`,
`DesignTheme`, `DesignLayer`, `DesignAction`, `DesignMutation`,
`DesignExportArtifact`, and `DesignToolProviderCapability`.

The DTOs must carry file/node identity, component/style/token metadata,
variable/theme aliasing, layer/document host state, mutation preconditions,
export artifact handles, webhook/watch cursors, provider capability hashes,
redaction profiles, and replay pointers. Raw provider payloads, private design
files, OAuth tokens, plugin manifests, raw pixels beyond policy, and
brand-specific workflows are rejected.

## Explicit Non-Goals

- Do not implement concrete Figma, Adobe, Penpot, Sketch, OAuth,
  plugin-runtime, desktop automation, provider client, design-to-code, brand,
  image generation, or storage providers in this research phase.
- Do not define product design workflows, design systems, brand token naming,
  asset pipelines, handoff flows, or application-specific design automation in
  OS layers.
- Do not expose raw provider JSON, plugin globals, desktop automation handles,
  OAuth tokens, or provider-specific routing as stable SDK contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, file handles, browser automation, media image/rendering, and
  secrets-reference handles provide reusable substrate.
- Current evidence does not prove design-tools DTOs, providers, SDK helpers,
  WASM ABI, tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
