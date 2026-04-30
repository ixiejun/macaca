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

## Driver Selection

Use the driver/tool requested by the task exactly.

- If the task says to use `opencode`, `opencode_execute`, or OpenCode, use `opencode_execute` for implementation and verification.
- If the task says to use `claude_code_execute` or Claude Code, use `claude_code_execute`.
- If no driver is specified, prefer `claude_code_execute` for normal backend coding tasks.
- Do not switch drivers mid-task unless the task explicitly allows it or the selected driver fails; if you switch after failure, report the reason in your review summary.

## opencode_execute
OpenCode driver for backend implementation tasks.
- Use when the user/task explicitly requests OpenCode or `opencode_execute`
- Always set `work_dir` to the app workspace root or backend project directory specified by the task
- Include exact file paths, acceptance criteria, and verification commands in prompts
- Use `opencode_resume` for multi-step work that should continue the same OpenCode session

## opencode_resume
Continue a previous OpenCode session.
- Use the `session_id` returned by `opencode_execute`
- Use for follow-up edits, debugging, or verification in the same task

## opencode_status
Check whether OpenCode is available.
- Use only when diagnosing driver availability or when an OpenCode task fails unexpectedly

## claude_code_execute
Claude Code driver for backend implementation tasks.
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
