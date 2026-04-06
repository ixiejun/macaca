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

## openspec

Primary tool for SDD specification management.

### Commands

```bash
# Initialize OpenSpec in a project
openspec init

# Create a new change proposal
openspec propose "feature-name"

# Full proposal with guided prompts
/opsx:propose

# Archive approved changes
openspec archive "feature-name"

# View current changes
openspec status
```

### Usage Patterns

1. **Starting a Feature**
   ```
   1. openspec init (if not already initialized)
   2. openspec propose "feature-name"
   3. Fill in the generated template files
   4. Review and refine
   ```

2. **After Implementation**
   ```
   1. Verify all tasks complete
   2. openspec archive "feature-name"
   ```

## claude_code_execute

Use for analysis tasks, not implementation.

```bash
# Analyze existing code
"Read the authentication flow in src/auth/ and summarize how it works"

# Check database schema
"Show me the current database migrations and explain the user table structure"

# Find patterns
"Search for API endpoint patterns in the backend code"
```

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

## Workflow

1. Receive feature request from Coordinator
2. Analyze requirements and gather context
3. Use openspec to create specifications
4. Review specs for completeness
5. Hand off to Frontend/Backend agents

## Workspace

The OS provides isolated workspace directories for each agent:

- **Shared workspace** (`shared/`): Shared by all agents. Place specifications, design docs, and architecture artifacts here so frontend/backend agents can access them.
- **Private workspace** (`agents/architect/`): Your private workspace. Use for drafts and working files before publishing to shared.

Always publish finalized specs and design documents to the shared workspace.
