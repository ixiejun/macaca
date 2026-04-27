---
name: figma-mcp
description: Fetch Figma design context (file metadata, node trees, styles, components) via the figma-developer-mcp stdio server. Use this to pull layout, spacing, colors, typography, and component structure from a Figma design URL so downstream agents can reason about UI implementation.
metadata:
  openclaw:
    emoji: 🎨
    os: ["linux", "darwin", "win32"]
    primaryEnv: FIGMA_API_KEY
    requires:
      bins: ["npx"]
      env: ["FIGMA_API_KEY"]
    install:
      - id: npm-figma-developer-mcp
        kind: npm
        package: figma-developer-mcp
        bins: ["figma-developer-mcp"]
        label: Install figma-developer-mcp via npx (no global install needed)
---

# figma-mcp — Figma Design Context Skill

Pulls layout, styles, and component trees from Figma files via the community [`figma-developer-mcp`](https://www.npmjs.com/package/figma-developer-mcp) MCP server (stdio transport).

## Prerequisites

1. **Figma Personal Access Token** — generate at `Figma → Account Settings → Personal access tokens`.
2. Export as environment variable **before starting the backend** (the MCP child process inherits parent env automatically):
   ```bash
   export FIGMA_API_KEY=figd_xxxxxxxxxxxxxxxxxxxxxxxxxx
   ./scripts/restart-dev.sh
   ```

## Available MCP Tools

The `figma-developer-mcp` server exposes (names may vary by package version — call `list_tools` at runtime to confirm):

- **`get_figma_data`** — fetch a Figma file or a specific node by `fileKey` + `nodeId`. Returns a JSON representation of the design tree (frames, auto-layout, styles, text, components).
- **`download_figma_images`** — export image assets referenced by given node ids.

## Extracting fileKey / nodeId from a Figma URL

A typical URL looks like:

```
https://www.figma.com/file/<FILE_KEY>/<DesignName>?node-id=<NODE_ID>
https://www.figma.com/design/<FILE_KEY>/<DesignName>?node-id=<NODE_ID>
```

- `FILE_KEY` — the 22-char segment right after `/file/` or `/design/`
- `NODE_ID` — the query parameter `node-id` (e.g. `123%3A456` URL-decoded to `123:456`)

## Typical Workflow

1. Parse the Figma URL the user provided → extract `fileKey` and (optionally) `nodeId`.
2. Call `get_figma_data({ fileKey, nodeId, depth: 3 })` to pull the node subtree.
3. Inspect the returned JSON for:
   - **Layout**: frame size, auto-layout direction, padding, gap
   - **Typography**: text styles (font family, size, weight, line-height)
   - **Color palette**: fills, strokes, effect colors
   - **Component structure**: nested frames, instances, variants
4. Produce a concise design-context note under `shared/design-context.md` so downstream agents (frontend implementers) can read it.

## Guardrails

- If `FIGMA_API_KEY` is missing, the skill's MCP server will fail to list tools — the runtime will surface `missing_env` in the skill status. Set the env var and restart the backend.
- Do NOT paste the API key into chat / issues / proposals; it must only live in the shell environment or local `.env` loaded by `restart-dev.sh`.
- Prefer small `depth` (2–3) for exploratory calls to avoid multi-MB responses. Drill into specific nodeIds for detail.
