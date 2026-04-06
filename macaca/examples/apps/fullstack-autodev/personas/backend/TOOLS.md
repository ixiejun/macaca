# Backend Agent Tools

## Task Board

You have a task board. Use these tools to manage your work:
- **list_my_tasks**: Check your board for assigned tasks
- **claim_task**: Claim the highest-priority pending task
- **start_task**: Mark a claimed task as in-progress
- **update_task_progress**: Report progress on current task
- **submit_task_for_review**: Submit completed work for Plan Agent review

When idle, check your board with `list_my_tasks`. If tasks are available, claim and execute them.
After completing a task, always call `submit_task_for_review` with a summary of what you did.

## claude_code_execute
Primary tool for writing Go backend code.
- Always set work_dir to the backend project directory
- Include the OpenSpec spec content in prompts for API contract context
- Use session continuation for multi-step implementations

## claude_code_resume
Continue a previous Claude Code session.
- Use for multi-step backend implementations
- Maintain session state across related API endpoint implementations

## golang
Go development utilities.
- `build`: Compile the Go project
- `test`: Run tests (`go test ./...`)
- `run`: Start the server locally
- `mod`: Manage dependencies (`go mod tidy`, `go mod download`)
- Always run `build` and `test` after implementation to verify correctness

## Workspace

The OS provides isolated workspace directories for each agent:

- **Shared workspace** (`shared/`): Shared by all agents. Place all project source files, APIs, and deliverables here so other agents can access them.
- **Private workspace** (`agents/backend/`): Your private workspace. Use for temporary files, intermediate artifacts, build caches, and scratch work.

Always write project code to the shared workspace. Use your private workspace only for files that no other agent needs.
