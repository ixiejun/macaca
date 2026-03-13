# SDD Spec Phase

You are in the **Spec** phase of Spec-Driven Development (SDD).

## Prerequisites

Analysis phase should be complete with:
- Clear requirements documented
- Technical context understood
- Architecture decisions made

## Your Task

Create detailed specifications using OpenSpec that will guide implementation.

### ⚠️ IMPORTANT: Non-Interactive Usage

**NEVER run `openspec init` directly in shell** - it requires interactive input!

**Always use the openspec tool with these exact parameters:**

```
Tool: openspec
Input: {
  "action": "init",
  "work_dir": "/path/to/project",
  "args": ["--tools", "claude"]
}
```

The `--tools claude` flag is **required** to skip interactive prompts.

### 1. Initialize OpenSpec (if not already initialized)

Check if `.claude/commands/openspec/` exists. If not:

```
openspec(action="init", work_dir="<project_dir>", args=["--tools", "claude"])
```

This creates:
- `.claude/commands/openspec/` - Slash commands for Claude Code
- `openspec/` - Specs and changes directories

### 2. Create Change Proposal

**Use claude_code_execute, NOT direct file writes:**

```
Tool: claude_code_execute
Input: {
  "prompt": "Create an OpenSpec change proposal for [FEATURE]. Use /openspec:proposal slash command.",
  "work_dir": "/path/to/project"
}
```

Claude Code will:
1. Use the `/openspec:proposal` slash command
2. Create `openspec/changes/<change-id>/proposal.md`
3. Create `openspec/changes/<change-id>/tasks.md`
4. Create `openspec/changes/<change-id>/specs/`

### 3. Review Generated Artifacts

After proposal creation, verify:
- `openspec/changes/<change-id>/proposal.md` - Feature description
- `openspec/changes/<change-id>/tasks.md` - Ordered task breakdown
- `openspec/changes/<change-id>/specs/` - Spec deltas

### 4. Validate Proposal

```
Tool: openspec
Input: {
  "action": "validate",
  "work_dir": "/path/to/project",
  "args": ["<change-id>", "--strict"]
}
```

## Quick Reference

| Action | Tool | Parameters |
|--------|------|------------|
| Init | `openspec` | `action="init", args=["--tools", "claude"]` |
| Propose | `claude_code_execute` | `prompt="use /openspec:proposal for..."` |
| Validate | `openspec` | `action="validate", args=["<id>", "--strict"]` |
| Apply | `claude_code_execute` | `prompt="use /openspec:apply for..."` |
| Archive | `openspec` | `action="archive", args=["<id>"]` |

## Common Mistakes to Avoid

❌ **WRONG:**
```bash
openspec init  # This requires interactive input!
```

✅ **CORRECT:**
```
openspec(action="init", work_dir="/project", args=["--tools", "claude"])
```

## Output Format

```markdown
## Specification Complete

### Artifacts Created
- `openspec/changes/<change-id>/proposal.md` - [brief description]
- `openspec/changes/<change-id>/tasks.md` - [task count]
- `openspec/changes/<change-id>/specs/` - [spec files]

### Summary
[One-paragraph summary of the specification]

### Task Breakdown
1. **[Task 1]** - [Agent assignment]
2. **[Task 2]** - [Agent assignment]

### Ready for Implementation
✓ Specifications created and validated
```
