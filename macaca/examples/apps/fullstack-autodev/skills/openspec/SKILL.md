---
name: openspec
description: OpenSpec CLI reference for Spec-Driven Development (SDD). Manages specs via Claude Code integration — init generates slash commands, then Claude Code uses /openspec:proposal and /openspec:apply internally.
---
# OpenSpec — Spec-Driven Development

Complete reference for using OpenSpec with Claude Code. OpenSpec follows a three-stage workflow: propose specs, get approval, then implement.

## Workflow Overview

```
Step 1: openspec init --tools claude     → Generates Claude Code integration files
Step 2: claude_code_execute              → Uses /openspec:proposal to create change proposals
Step 3: (Human/Agent reviews proposal)
Step 4: claude_code_execute              → Uses /openspec:apply to implement approved changes
Step 5: openspec validate                → Verify implementation matches specs
```

## Step 1: Initialize OpenSpec

```
Tool: openspec
Input: {
  "action": "init",
  "work_dir": "/path/to/project",
  "args": ["--tools", "claude"]
}
```

**IMPORTANT:** Always pass `"args": ["--tools", "claude"]` to skip interactive prompts and generate Claude Code integration files.

This creates:
```
.claude/commands/openspec/
  proposal.md           ← /openspec:proposal slash command
  apply.md              ← /openspec:apply slash command
  archive.md            ← /openspec:archive slash command
CLAUDE.md               ← Claude Code project instructions
openspec/
  AGENTS.md             ← Full SDD workflow documentation
  project.md            ← Project context template
  changes/              ← Change proposals directory
  specs/                ← Approved specifications
```

## Step 2: Create Change Proposals

Use `claude_code_execute` — Claude Code will use the `/openspec:proposal` slash command internally:

```
Tool: claude_code_execute
Input: {
  "prompt": "I want to add [FEATURE DESCRIPTION]. Please create an OpenSpec change proposal for this feature using /openspec:proposal",
  "work_dir": "/path/to/project"
}
```

Claude Code reads the generated `.claude/commands/openspec/proposal.md` and:
1. Reviews existing specs and code
2. Creates `openspec/changes/<change-id>/proposal.md` — the change proposal
3. Creates `openspec/changes/<change-id>/tasks.md` — ordered task breakdown
4. Creates `openspec/changes/<change-id>/specs/` — spec deltas per capability
5. Runs `openspec validate <change-id> --strict` to verify

**Output example:**
```
openspec/changes/add-hello-world-endpoint/
  proposal.md
  tasks.md
  specs/hello-world-api/spec.md
```

## Step 3: Implement Approved Changes

Use `claude_code_execute` — Claude Code will use the `/openspec:apply` slash command:

```
Tool: claude_code_execute
Input: {
  "prompt": "Please apply the approved change 'add-hello-world-endpoint' using /openspec:apply. Implement all tasks in tasks.md sequentially.",
  "work_dir": "/path/to/project"
}
```

Claude Code reads `.claude/commands/openspec/apply.md` and:
1. Reads proposal.md, design.md, tasks.md
2. Works through tasks sequentially
3. Marks completed tasks in tasks.md

## Step 4: Validate Implementation

```
Tool: openspec
Input: {
  "action": "validate",
  "work_dir": "/path/to/project",
  "args": ["<change-id>", "--strict"]
}
```

If validation fails, use `claude_code_execute` to fix issues and re-validate.

## Step 5: Archive Completed Changes

```
Tool: openspec
Input: {
  "action": "archive",
  "work_dir": "/path/to/project",
  "args": ["<change-id>"]
}
```

## Other Useful Commands

### List changes and specs
```
Tool: openspec
Input: { "action": "list", "work_dir": "/path/to/project" }
```

### Show a specific change
```
Tool: shell
Input: { "command": "cd /path/to/project && openspec show <change-id>" }
```

### Update OpenSpec instructions
```
Tool: openspec
Input: { "action": "update", "work_dir": "/path/to/project" }
```

## Complete Example Flow

```
1. openspec(action="init", work_dir="/tmp/todo-app", args=["--tools", "claude"])

2. claude_code_execute(
     prompt="I want to build a TODO app with CRUD operations and user authentication.
             Please create an OpenSpec change proposal using /openspec:proposal",
     work_dir="/tmp/todo-app"
   )

3. claude_code_execute(
     prompt="Please apply the approved change using /openspec:apply.
             Implement all tasks sequentially.",
     work_dir="/tmp/todo-app"
   )

4. openspec(action="validate", work_dir="/tmp/todo-app", args=["--strict"])
```

## Key Rules

- **Always init first** with `--tools claude` before any other operations
- **Proposals via Claude Code** — use `claude_code_execute` with `/openspec:proposal`, not direct file writes
- **Implementation via Claude Code** — use `claude_code_execute` with `/openspec:apply`
- **If `claude_code_execute` fails, STOP** — report the error, never fallback to `file_write` for source code
- **Validate after implementation** — always run `openspec validate` to verify
- **One change at a time** — complete one change proposal before starting the next

## Tech Stack Conventions

### Frontend (Next.js)
- Next.js 14+ with App Router
- shadcn/ui (Radix UI + Tailwind CSS)
- TypeScript strict mode
- React Hook Form + Zod validation

### Backend (Go)
- chi v5 router
- PostgreSQL with pgx driver
- cmd/internal/pkg layout
- Go standard testing + testify
