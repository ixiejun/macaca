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

Fetch design context from Figma URLs via the `figma-developer-mcp` MCP server.
The server auto-starts on first call (stdio transport); the backend must have
`FIGMA_API_KEY` exported in its environment.

### Available tools (exact names will be shown in your tool list at runtime)

- `get_figma_data` — fetch a Figma file or node subtree
  - `fileKey` (string, required): 22-char id from the Figma URL
  - `nodeId` (string, optional): specific node id (URL-decoded, e.g. `123:456`)
  - `depth` (int, optional): recursion depth; use 2–3 for overview, deeper for detail
- `download_figma_images` — export image assets for given nodeIds

### Extract fileKey / nodeId from a Figma URL

```
https://www.figma.com/file/<FILE_KEY>/Design-Name?node-id=<NODE_ID>
https://www.figma.com/design/<FILE_KEY>/Design-Name?node-id=<NODE_ID>
                             ^^^^^^^^              ^^^^^^^
```

`node-id` in the URL is URL-encoded (`%3A` = `:`). Decode before passing.

### Typical invocation

```json
{
  "tool": "get_figma_data",
  "input": {
    "fileKey": "abcDEF1234567890",
    "nodeId": "123:456",
    "depth": 3
  }
}
```

After receiving the JSON tree, summarize:
- **Layout**: frame size, auto-layout direction, padding, spacing
- **Typography**: font family / size / weight
- **Colors**: fills / strokes / effects
- **Component structure**: instances, variants, nested frames

Publish the summary to `shared/design-context.md` for downstream implementers.

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
