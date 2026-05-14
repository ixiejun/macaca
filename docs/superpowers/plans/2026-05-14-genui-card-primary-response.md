# GenUI Card Primary Response Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make GenUI session surfaces the primary visible response when a WASM app emits `ui.render`, while keeping raw WASM execution JSON out of the main assistant bubble.

**Architecture:** The Web shell remains generic. A small frontend helper recognizes schema-shaped WASM execution acknowledgements and suppresses only those that contain stored GenUI render evidence. The existing `GenUiRenderer` continues to render the app-owned card surface fetched by session id.

**Tech Stack:** Next.js 16, React 19, TypeScript, existing CSS in `frontend/app/globals.css`.

---

### Task 1: Generic WASM Execution Receipt Filtering

**Files:**
- Modify: `frontend/app/chat/[appId]/page.tsx`

- [x] **Step 1: Add a helper near the chat event helpers**

Add a pure helper:

```ts
function genUiStoredFromWasmReceipt(content: string | undefined): boolean {
  if (!content?.startsWith('WASM execution completed')) return false;
  const jsonStart = content.indexOf('{');
  if (jsonStart < 0) return false;
  try {
    const parsed = JSON.parse(content.slice(jsonStart)) as {
      output?: { host_command_results?: Array<{ metadata?: Record<string, unknown> }> };
    };
    return parsed.output?.host_command_results?.some((result) => (
      result.metadata?.reason_code === 'ui_render_stored'
    )) === true;
  } catch {
    return false;
  }
}
```

- [x] **Step 2: Use the helper in SSE assistant handling**

When an `assistant` event contains a GenUI stored WASM receipt, set the visible content to `Application surface rendered` and keep the raw receipt in `trace_steps`.

- [x] **Step 3: Use the helper for legacy content events**

Apply the same visible-content rule to `content` events so both SSE shapes behave identically.

### Task 2: Card Styling Polish

**Files:**
- Modify: `frontend/app/globals.css`

- [x] **Step 1: Improve GenUI card hierarchy**

Make `.genui-card` feel like a compact analysis card: structured spacing, stronger title, readable body text, and stable responsive width.

- [x] **Step 2: Improve list/table readability**

Add subtle row/list styling so buy/sell point, support/resistance, and risk fields scan as grouped facts.

### Task 3: Verification

**Files:**
- Existing frontend and running dev server.

- [x] **Step 1: Run lint**

Run:

```bash
npm run lint
```

Expected: exit 0.

- [x] **Step 2: Smoke the running chat page**

Open the existing chat page or use the running dev server. Expected visible result: the raw `WASM execution completed {...}` text no longer dominates the assistant response when a GenUI surface is emitted; the GenUI card remains visible.
