# AGENTS.md

Project-wide notes for AI agents. Macaca-specific engineering rules live in `macaca/AGENTS.md`.

## Cursor Cloud specific instructions

### What this repo is

Monorepo for **Macaca Agent OS** (Rust workspace under `macaca/`) plus application packages (`apps/`, `macaca/examples/apps/`). The Next.js dashboard (`frontend/`) is a **separate git repo** and is not checked into this workspace.

### System dependencies (one-time on fresh VMs)

- **Rust stable ≥ 1.96** — the VM image may ship Rust 1.83; run `rustup default stable` before building. Crate `time` requires edition 2024.
- **`libssl-dev` + `pkg-config`** — required to compile `openssl-sys` (NATS/TLS).
- **Node.js 18+** — for `apps/codex-wasm-workbench/ui` and optional tooling.

### Update script vs manual setup

The VM **update script** only refreshes language dependencies (`rustup`, `cargo fetch`, `npm ci`). It does **not** install apt packages or start servers.

### Build & test (see `macaca/README.md`)

| Task | Command (from repo root) |
|------|--------------------------|
| Build CLI + web | `cd macaca && cargo build --bin macaca --bin macaca-web-server` |
| Typecheck | `cd macaca && cargo check` |
| Unit/integration tests | `cd macaca && cargo test --workspace` |
| Codex UI deps | `cd apps/codex-wasm-workbench/ui && npm ci` |

`cargo clippy -- -D warnings` currently fails on pre-existing warnings; use `cargo check` for CI-style validation unless you are fixing clippy debt.

One integration test (`frontend_persona_loads`) fails when `frontend/` is absent — expected in this checkout.

### Running services (manual — not in update script)

| Service | Start | URL |
|---------|-------|-----|
| **Macaca Web API** | `cd macaca && ./target/debug/macaca-web-server --port 3001` (or `cargo run --bin macaca -- web --port 3001`) | `http://localhost:3001` |
| **Codex workbench UI** | `cd apps/codex-wasm-workbench/ui && npm run dev -- --host 0.0.0.0 --port 5173` | `http://localhost:5173` |
| **Next.js dashboard** | Clone `frontend/` into repo root, then `npm install && npm run dev` | `http://localhost:3000` |

Use **tmux** for long-running dev servers in Cloud Agent VMs.

### Workspace / app discovery gotcha

`macaca web` discovers applications only under `{workspace.root_dir}/apps` (see `macaca/config/default.toml`). The committed config points at a macOS-specific path (`/Users/quantum/.macaca/workspaces`). On Linux/cloud VMs:

1. Create a local workspace, e.g. `/workspace/.macaca-workspace/apps/`
2. Symlink apps into it: `fullstack-autodev` → `macaca/examples/apps/fullstack-autodev`, `codex-wasm-workbench` → `apps/codex-wasm-workbench`
3. Override config via env when starting the web server: `AOS_WORKSPACE__ROOT_DIR=/workspace/.macaca-workspace` (prefix `AOS`, nested keys use `__`)

If apps still show as empty, confirm the scan path in logs: `Apps discovered from workspace application directory`.

### Hello-world API checks (backend alive)

```bash
curl -s http://localhost:3001/api/status | jq .
curl -s http://localhost:3001/api/apps | jq .
curl -s -X POST http://localhost:3001/api/apps/reload | jq .
```

A healthy boot returns HTTP 200 from `/api/status` with `version`, `llm_provider`, and service-runtime inspection in logs.

### Optional services

- **Milvus** (vector memory): `macaca/infra/milvus/docker-compose.yml` — not required for API smoke tests; set `memory.vector.backend = "none"` to skip.
- **LLM keys**: configured in `macaca/config/default.toml` or env; live agent chat needs a working provider.

### Logs

- Macaca web (if tee'd): `/tmp/macaca-web.log`
- Codex Vite: `/tmp/codex-ui.log`
- Structured JSON logs: `macaca/logs/`

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **macaca** (69078 symbols, 119690 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/macaca/context` | Codebase overview, check index freshness |
| `gitnexus://repo/macaca/clusters` | All functional areas |
| `gitnexus://repo/macaca/processes` | All execution flows |
| `gitnexus://repo/macaca/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
