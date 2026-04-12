## Development

### Project Structure

```
macaca/
├── Cargo.toml              # Workspace manifest
├── config/
│   └── default.toml        # Main configuration
├── crates/
│   ├── macaca-web/         # HTTP API server
│   ├── macaca-kernel/      # Agent scheduling & execution
│   ├── macaca-runtime/     # Agentic loop
│   ├── macaca-task/        # TaskBoard, TodoStore, loops
│   ├── macaca-llm/         # LLM abstraction
│   ├── macaca-tools/       # Agent tools
│   ├── macaca-proto/       # Shared types
│   └── ...                 # (21 crates total)
├── examples/
│   └── apps/               # Example applications
│       └── fullstack-autodev/
│           ├── app.yaml    # App manifest
│           ├── agents/     # Agent definitions
│           ├── IDENTITY.md
│           ├── TOOLS.md
│           └── SOUL.md
├── docs/                   # Documentation
│   ├── SYSTEM_OVERVIEW.md
│   └── SYSTEM_AUDIT.md
├── frontend/               # Next.js frontend
│   ├── app/
│   ├── components/
│   └── lib/
└── src/                    # Legacy source files
```

### Creating a New Application

1. Create app directory in `examples/apps/your-app/`
2. Create `app.yaml` manifest:

```yaml
name: "your-app"
version: "1.0.0"
description: "Your application description"
agents:
  - name: "coordinator"
    persona: "IDENTITY.md"
    tools: "TOOLS.md"
    soul: "SOUL.md"
    capabilities:
      - name: "planning"
      - name: "delegation"
  - name: "worker"
    persona: "IDENTITY.md"
    tools: "TOOLS.md"
    soul: "SOUL.md"
    capabilities:
      - name: "coding"
      - name: "analysis"
```

3. Create agent definition files (`IDENTITY.md`, `TOOLS.md`, `SOUL.md`)
4. Restart the server or call `POST /api/apps/reload`

### Adding a Skill

Knowledge skills (from `SKILL.md`):

```markdown
# Skill Name

## Description
Brief description of what this skill does.

## Usage
How to use this skill.

## Example
Example of the skill in action.
```

Executable skill tools (from `skill.yaml` or code):

```yaml
name: "my-skill"
description: "My custom skill tool"
command: "python3 scripts/my-skill.py"
```

### Testing

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test '*'

# Run with output
cargo test -- --nocapture
```

