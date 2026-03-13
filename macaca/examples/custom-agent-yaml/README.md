# Code Review Agent Example

A declarative code review agent for Agent OS.

## Files

- `code-review-agent.yaml` - Agent configuration with code review and static analysis capabilities
- `app-manifest.yaml` - App manifest that loads the agent

## Usage

```bash
aos install code-review-agent.yaml
aos run "Review src/main.rs for security issues"
```
