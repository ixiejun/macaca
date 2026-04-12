## API Documentation

The API server runs on port 3001. All endpoints return JSON.

### System Status

**GET** `/api/status`

```json
{
  "version": "0.1.0",
  "agent_count": 5,
  "app_count": 1,
  "llm_provider": "openai"
}
```

### Applications

**GET** `/api/apps` - List all applications

```json
[
  {
    "id": "uuid",
    "name": "fullstack-autodev",
    "status": "Running",
    "agent_count": 5,
    "description": "Fullstack AutoDev application",
    "icon": "cube"
  }
]
```

**GET** `/api/apps/{id}` - Get single app info

**GET** `/api`/apps/{id}/agents` - List agents for an app

**GET** `/api/apps/{id}/agents/stream` - SSE stream of agent statuses (real-time)

**POST** `/api/apps/reload` - Hot-reload apps from disk

### Chat & Sessions

**POST** `/api/chat` - Send chat message

```json
{
  "app_id": "uuid",
  "message": "Create a REST API for a todo app",
  "session_id": "optional-uuid"
}
```

**POST** `/api/chat/v2` - Chat v2 endpoint with enhanced features

**POST** `/api/chat/stop` - Cancel running chat

**GET** `/api/sessions` - List all sessions

**GET** `/api/sessions/{id}/events` - Get persisted event log for a session

**GET** `/api/sessions/{id}/run-trace` - Get execution trace checkpoints

### Task Board

**GET** `/api/apps/{app_id}/todos` - List todos (optional `?session_id=...` filter)

**GET** `/api/apps/{app_id}/todos/progress` - Get task progress summary

```json
{
  "total": 10,
  "pending":": 2,
  "assigned": 0,
  "in_progress": 3,
  "pending_review": 1,
  "completed": 4,
  "blocked": 0,
  "failed": 0,
  "cancelled": 0,
  "all_done": false
}
```

**GET** `/api/apps/{app_id}/todos/{agent_name}` - List agent's task board

**GET** `/api/apps/{app_id}/goals` - List goals

**POST** `/api/apps/{app_id}/goals` - Create a new goal

```json
{
  "description": "Build user authentication system"
}
```

### Schedules

**GET** `/api/apps/{app_id}/schedules` - List all schedules

**POST** `/api/apps/{app_id}/schedules` - Create a schedule

```json
{
  "name": "daily-report",
  "cron_expr": "0 9 * * *",
  "action": {
    "kind": "create_goal",
    "description": "Generate daily report"
  }
}
```

**GET** `/api/apps/{app}/schedules/{id}` - Get schedule details

**DELETE** `/api/apps/{app}/schedules/{id}` - Delete a schedule

**PUT** `/api/apps/{app}/schedules/{id}/toggle` - Enable/disable schedule

```json
{
  "enabled": false
}
```

### Skills

**GET** `/api/skills` - List available skills

```json
[
  {
    "name": "code-review",
    "description": "Perform code review on a file"
  }
]
```

### Metrics

**GET** `/metrics` - Prometheus metrics

