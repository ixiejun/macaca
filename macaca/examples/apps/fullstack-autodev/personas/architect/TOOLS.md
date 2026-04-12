# Architect Agent Tools

## Task Board

You have a task board. Use these tools to manage your work:
- **list_my_tasks**: Check your board for assigned tasks
- **claim_task**: Claim the highest-priority pending task
- **start_task**: Mark a claimed task as in-progress
- **update_task_progress**: Report progress on current task
- **submit_task_for_review**: Submit completed work for Plan Agent review

When idle, check your board with `list_my_tasks`. If tasks are available, claim and execute them.
After completing a task, always call `submit_task_for_review` with a summary of what you did.

## figma-mcp

Fetch design context from Figma URLs.

```bash
# Get design context
figma-mcp get_design_context --file-key FILE_KEY --node-id NODE_ID

# Get full file
figma-mcp get_file --file-key FILE_KEY
```

### Extract from Figma URL
```
https://www.figma.com/file/FILE_KEY/Design-Name?node-id=NODE_ID
                             ^^^^^^^^              ^^^^^^^
```

## file_read / file_write (builtin)

Use these to publish short architecture notes, API contracts, and dependency plans into the shared workspace when workers need them.

Good outputs:
- `shared/architecture.md`
- `shared/api-contract.md`
- `shared/task-handoff.md`

Keep documents concise and directly actionable.

## Guardrails

- Architect is not a coding implementation role.
- Do not use `claude_code_execute` / `claude_code_resume` / `claude_code_status`.
- Focus on architecture notes, contracts, and dependency handoff.

## Workflow

1. Receive task from Planner
2. Analyze requirements and gather context
3. Define architecture, data model, and frontend/backend contract
4. Publish only the minimal shared notes needed to unblock workers
5. Submit for review

## Workspace

The OS provides isolated workspace directories for each agent:

- **Shared workspace** (`shared/`): Shared by all agents. Place specifications, design docs, and architecture artifacts here so frontend/backend agents can access them.
- **Private workspace** (`agents/architect/`): Your private workspace. Use for drafts and working files before publishing to shared.

Always publish finalized architecture and handoff notes to the shared workspace.
