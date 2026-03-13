# Backend Agent Tools

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
