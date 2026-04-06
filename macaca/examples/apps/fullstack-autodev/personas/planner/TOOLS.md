# Planner Agent Tools

## Task Management

### create_todo
Create and assign a task to a specific agent's board.
```json
{
  "agent": "backend",
  "title": "Implement REST API endpoints",
  "description": "Create CRUD endpoints for the blog post resource...",
  "priority": 8,
  "acceptance_criteria": ["GET /api/posts returns 200", "POST creates new post"]
}
```

### review_todo
Review a task submitted by an agent. Check if acceptance criteria are met.
```json
{
  "task_id": "uuid",
  "agent": "backend",
  "passed": true,
  "feedback": "All endpoints implemented correctly, tests pass"
}
```

### check_todo_progress
Check overall progress of all tasks across all agents.
```json
{}
```

### reassign_task
Move a task from one agent to another.
```json
{
  "task_id": "uuid",
  "current_agent": "frontend",
  "new_agent": "backend"
}
```

### create_goal
Create a new high-level goal for decomposition.
```json
{
  "description": "Build user authentication with JWT"
}
```

## Workspace

The OS provides isolated workspace directories for each agent:

- **Shared workspace** (`shared/`): Shared by all agents. This is the main collaboration space.
- **All agent workspaces** (`agents/`): As a supervisor, you can read all agent private workspaces to monitor progress and collect artifacts.

Use the shared workspace to publish plans, task breakdowns, and coordination documents.
