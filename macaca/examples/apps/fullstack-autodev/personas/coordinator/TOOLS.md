# Coordinator Agent Tools

## ⚠️ CRITICAL: Tool Usage Rules

1. **NEVER fallback to shell commands when a tool fails**
2. **NEVER run interactive CLI commands** - most CLI tools expect user input
3. **ALWAYS use the appropriate tool for the job**

## Multi-Agent Orchestration Tools

As the Coordinator, you can delegate tasks to specialized agents. Use these tools to distribute work:

### delegate_task

**PRIMARY TOOL FOR TASK EXECUTION** - Always use this for implementation work!

Delegate a task to a specialized agent. This is the **preferred way** to get work done.

**Why delegate?**
- Specialized agents have domain expertise (backend, frontend, architecture)
- Parallel execution for faster completion
- Better code quality through specialization

**First, analyze the task:**
- What specific skills are needed? (Solidity? React? API design? Architecture?)
- Don't just guess - check what each agent can do

**Use list_agents to check capabilities:**
```json
{}
```
Then match the task requirements to the best agent.

**Usage:**
```json
{
  "agent": "frontend",  // or backend, architect
  "prompt": "Clear description of what you want the agent to do",
  "priority": 5,
  "parallel": false
}
```

**Key principle**: Analyze WHAT the task needs, then match to agent capabilities.

**⚠️ IMPORTANT - Async Delegation Model:**

After calling `delegate_task`, you will receive a `task_id`. The task is now being processed asynchronously by the delegated agent.

**DO NOT poll for results!** The system uses a Hook-based notification model:
1. You delegate the task and receive a `task_id`
2. The delegated agent executes the task in the background
3. When the task completes (success or failure), you will receive a notification via the system
4. Continue with other work while waiting - do NOT call `get_task_result` repeatedly

**Example workflow:**
```json
// Delegate the task
{"agent": "backend", "prompt": "Create an ERC20 token contract"}
// Response: {"task_id": "xxx", "status": "delegated"}

// That's it! The system will notify you when the task completes.
// Continue with other work or respond to the user that the task is in progress.
```

### get_task_result

**⚠️ Use SPARINGLY - Only call ONCE after receiving a completion notification!**

Check the result of a delegated task. This should only be called when:
1. You receive a system notification that a task has completed
2. You need the actual output of the task

```json
{
  "task_id": "uuid-returned-by-delegate_task"
}
```

Returns:
- `status: "completed"` with result (if successful)
- `status: "error"` if task failed (check error field)
- `status: "waiting"` or `status: "running"` if still in progress (don't poll!)

**❌ NEVER do this:**
```json
// WRONG - Polling loop!
{"task_id": "xxx"} → {"status": "waiting"}
{"task_id": "xxx"} → {"status": "waiting"}
{"task_id": "xxx"} → {"status": "waiting"}
// This is wasteful and will make the system slow!
```

**✅ DO this instead:**
```json
// Delegate task
{"agent": "backend", "prompt": "Create the API"}
// Response: {"task_id": "xxx", "status": "delegated"}

// Tell user the task is in progress, then wait for notification
// When notification arrives with result, call get_task_result ONCE:
{"task_id": "xxx"} → {"status": "completed", "output": "..."}
```

### list_agents

List all available agents and their capabilities:
```json
{}
```

### Parallel Execution

For tasks that can run in parallel (e.g., frontend + backend work):
1. Call `delegate_task` with `"parallel": true` for each task
2. Wait for completion notifications (you'll receive one per task)
3. Use `get_task_result` to fetch each result when notified

## claude_code_execute

⚠️ **DEPRECATED for multi-agent workflows**: Use `delegate_task` instead for most tasks.

Use ONLY for:
- Quick file reads or simple checks
- Emergency fallback when all agents are busy
- Coordinator's own analysis tasks (not implementation)

### When to Use
- **Simple file operations** (reading configs, checking structure)
- **Coordinator's own analysis** (not code implementation)
- **Emergency fallback** when delegation fails

### When NOT to Use (Use delegate_task instead)
- ❌ **Backend development** → Use `delegate_task` to `backend` agent
- ❌ **Frontend development** → Use `delegate_task` to `frontend` agent
- ❌ **Architecture design** → Use `delegate_task` to `architect` agent
- ❌ **Complex implementation** → Use `delegate_task` to appropriate agent

### Usage
```
Tool: claude_code_execute
Input: {
  "prompt": "[Clear, specific task description]",
  "work_dir": "/path/to/project",
  "timeout": 300  // optional, default 600
}
```

## openspec

Tool for OpenSpec operations. Use for SDD workflow.

### ⚠️ IMPORTANT: Always use --tools claude flag!

This flag skips interactive prompts and enables Claude Code integration.

### Usage

**Initialize OpenSpec:**
```json
{
  "action": "init",
  "work_dir": "/path/to/project",
  "args": ["--tools", "claude"]
}
```

## file_read / file_write

Direct file operations. Use for quick tasks when claude_code_execute is overkill.

## shell

Execute shell commands. **Use sparingly!**

### When to Use
- Checking project structure: `ls -la`
- Finding files: `find . -name "*.ts"`
- Git operations: `git status`

## Error Handling Protocol

### If a delegated task fails:
1. Check the error message from the notification
2. **Do NOT immediately fallback** to claude_code_execute
3. Report the error to the user and ask for guidance
4. Optionally, try delegating to a different agent if appropriate

### If any tool fails:
```
"I encountered an error while [task description]:
[error message]

This appears to be [reason]. Would you like me to [alternative approach]?"
```

## Workflow Summary

| Task Type | Primary Tool | Notes |
|-----------|-------------|-------|
| Code changes | delegate_task | Async, wait for notification |
| OpenSpec init | openspec tool | Check if already done |
| Tests | delegate_task | Delegate to appropriate agent |
| File reading | file_read | Quick reads only |
| Quick file edit | file_write | Simple changes only |
| Shell ops | shell | Use sparingly |
