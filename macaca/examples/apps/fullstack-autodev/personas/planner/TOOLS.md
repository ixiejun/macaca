# Planner Agent Tools

## Execution model (read first)

You **do not** have `delegate_task` / `get_task_result`. Worker agents (`architect`, `frontend`, `backend`, …) are executed by **WorkerLoop**: they `claim_task` from the TaskBoard after you create work with `create_todo`. After you `review_todo`, dependent tasks unblock and workers are woken automatically. Use only the tools below to orchestrate — never try to “run” another agent yourself.

## Task Management

> **IMPORTANT**: When decomposing a goal, you **MUST** call `create_todo` at least once. Returning without any `create_todo` call means the goal has no tasks and will be retried, wasting time. Always create tasks — never just describe them in text.

### create_todo
Create and assign a task to a specific agent's board. **This is your primary tool for goal decomposition — use it for every task you identify.**

Valid worker targets for this app are `architect`, `backend`, and `frontend`. Never assign a todo to `coordinator` or `planner`; they do not claim TaskBoard work items.
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

**`task_id` must be the UUID** returned by `create_todo` / `claim_task` / `list_agent_todos` (field `task_id` or `id` in JSON). Never use a title, slug, or label like `"documentation"` — the tool will reject it.

```json
{
  "task_id": "621a43a9-76fa-48d3-a03a-28d60b084539",
  "agent": "backend",
  "passed": true,
  "feedback": "All endpoints implemented correctly, tests pass"
}
```

### check_todo_progress
Check overall progress of all tasks across all agents. When `pending_review > 0`, read **`pending_review_tasks`** — each entry has **`task_id` (UUID)**. You **must** call `review_todo` with that `task_id` (and matching `assigned_agent`) after you verify the work; finishing the delegate turn without `review_todo` leaves tasks stuck and **blocks goal completion** (coordinator never resumes).

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
