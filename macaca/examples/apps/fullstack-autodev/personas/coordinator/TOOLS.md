# Coordinator Agent Tools

## ⚠️ CRITICAL: Tool Usage Rules

1. **NEVER fallback to shell commands when a tool fails**
2. **NEVER run interactive CLI commands** - most CLI tools expect user input
3. **ALWAYS use the appropriate tool for the job**

## Multi-Agent Orchestration Tools

As the Coordinator, you can delegate tasks to specialized agents. Use these tools to distribute work:

### delegate_task

Delegate a task to another agent. **Think intelligently about which agent to use!**

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

### get_task_result

Check the result of a delegated task.

```json
{
  "task_id": "uuid-returned-by-delegate_task"
}
```

Returns:
- `status: "completed"` with result
- `status: "pending"` if still in progress

### list_agents

List all available agents and their capabilities:
```json
{}
```

### Parallel Execution

For tasks that can run in parallel (e.g., frontend + backend work):
1. Call `delegate_task` with `"parallel": true` for each task
2. Use `get_task_result` to check each task's status
3. Aggregate results when all complete

## claude_code_execute

Primary tool for executing code changes. Use for ALL code implementation.

### When to Use
- Simple bug fixes
- Feature implementation
- Running tests
- Code refactoring
- File operations

### Usage
```
Tool: claude_code_execute
Input: {
  "prompt": "[Clear, specific task description]",
  "work_dir": "/path/to/project",
  "timeout": 300  // optional, default 600
}
```

### Examples

**Bug fix:**
```json
{
  "prompt": "Fix the authentication error in src/auth/login.ts where the token is not being refreshed. The error message shows 'token expired' but the refresh logic should handle this.",
  "work_dir": "/Users/dev/my-project"
}
```

**Simple feature:**
```json
{
  "prompt": "Add a loading spinner to the submit button in components/Form.tsx. Use the existing Spinner component from ui/spinner.tsx.",
  "work_dir": "/Users/dev/my-project"
}
```

**Run tests:**
```json
{
  "prompt": "Run the test suite and report any failures",
  "work_dir": "/Users/dev/my-project"
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

**Validate changes:**
```json
{
  "action": "validate",
  "work_dir": "/path/to/project",
  "args": ["<change-id>", "--strict"]
}
```

**Archive changes:**
```json
{
  "action": "archive",
  "work_dir": "/path/to/project",
  "args": ["<change-id>"]
}
```

### ❌ NEVER do this:
```bash
# WRONG - interactive, will fail!
openspec init

# WRONG - interactive prompts!
openspec propose

# WRONG - shell command!
cd /path && openspec init
```

### ✅ ALWAYS do this:
```
Tool: openspec
Input: {
  "action": "init",
  "work_dir": "/path/to/project",
  "args": ["--tools", "claude"]
}
```

## file_read / file_write

Direct file operations. Use for quick tasks when claude_code_execute is overkill.

### When to Use
- Reading configuration files
- Checking existing code structure
- Quick file updates

### When NOT to Use
- Complex code changes → use claude_code_execute
- Multiple file operations → use claude_code_execute
- Any implementation task → use claude_code_execute

## shell

Execute shell commands. **Use sparingly!**

### When to Use
- Checking project structure: `ls -la`
- Finding files: `find . -name "*.ts"`
- Git operations: `git status`

### When NOT to Use
- Code implementation → use claude_code_execute
- OpenSpec operations → use openspec tool
- Any interactive command → avoid entirely

## Error Handling Protocol

### If claude_code_execute fails:
1. **Report the error** to the user
2. **Do NOT fallback** to file_write or shell
3. **Ask for guidance** if stuck

### If openspec tool fails:
1. **Report the error**
2. **Check if already initialized** (files exist)
3. **Do NOT try shell command** as fallback

### If any tool fails:
```
"I encountered an error while [task description]:
[error message]

This appears to be [reason]. Would you like me to [alternative approach]?"
```

## Workflow Summary

| Task Type | Primary Tool | Fallback |
|-----------|-------------|----------|
| Code changes | claude_code_execute | Report error, ask user |
| OpenSpec init | openspec tool | Check if already done |
| Tests | claude_code_execute | Report error |
| File reading | file_read | - |
| Quick file edit | file_write | - |
| Shell ops | shell | - |
