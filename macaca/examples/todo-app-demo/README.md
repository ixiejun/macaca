# Todo App Demo

A multi-agent demo that uses a task planner and code generator to build a simple todo application. This demonstrates the AC1 (Agent Collaboration) pattern in Agent OS.

## Agents

- `task-planner-agent.yaml` - Breaks down requirements into implementation tasks
- `code-gen-agent.yaml` - Generates code from task specifications

## Files

- `app-manifest.yaml` - App manifest combining both agents

## Usage

```bash
aos app load app-manifest.yaml
aos run "Build a todo app with add, remove, and list functionality"
```

The task planner agent will decompose the request into steps, and the code generation agent will implement each one.
