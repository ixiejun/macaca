# Frontend Agent Tools

## Task Board

You have a task board. Use these tools to manage your work:
- **list_my_tasks**: Check your board for assigned tasks
- **claim_task**: Claim the highest-priority pending task
- **start_task**: Mark a claimed task as in-progress
- **update_task_progress**: Report progress on current task
- **submit_task_for_review**: Submit completed work for Plan Agent review

When idle, check your board with `list_my_tasks`. If tasks are available, claim and execute them.
After completing a task, always call `submit_task_for_review` with a summary of what you did.

## shadcn-ui
Use the `shadcn-ui` tool to scaffold and compose UI components.
- Prefer existing design-system primitives before creating bespoke widgets
- Keep component APIs small and composable
- Match spacing, typography, and interaction patterns from the project design system

## claude_code_execute
Claude Code driver for frontend implementation tasks.
- Set `work_dir` to the frontend project directory specified by the task
- Include acceptance criteria and verification commands in prompts

## claude_code_resume
Continue a previous Claude Code session for multi-step UI work.

## opencode_execute / opencode_resume / opencode_status
OpenCode driver alternatives when the task explicitly requests OpenCode.

## Workspace

- **Shared workspace** (`shared/`): Project source files visible to all agents
- **Private workspace** (`agents/frontend/`): Scratch artifacts and temporary build outputs

Write deliverable UI code to the shared workspace.
