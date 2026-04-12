
---

## Technology Stack

### Backend (Rust)
- **Runtime**: Tokio async runtime
- **Web Framework**: Axum HTTP server
- **Database**: redb (embedded)
- **Vector Store**: Milvus (optional)
- **IPC**: NATS
- **Serialization**: serde, serde_json, bincode
- **Logging**: tracing with OpenTelemetry support

### Frontend (Next.js)
- **Framework**: Next.js 16
- **UI Library**: React 19
- **Styling**: Tailwind CSS 4
- **Icons**: Lucide React

### Key Rust Crates
| Crate | Purpose |
|-------|---------|
| `macaca-web` | HTTP API server (port 3001) |
| `macaca-kernel` | Agent scheduling, executor, fork management |
| `macaca-runtime` | Agentic loop, context window |
| `macaca-task` | TaskBoard, TodoStore, PlanLoop, WorkerLoop, Scheduler |
| `macaca-llm` | LLM abstraction and resilient wrapper |
| `macaca-tools` | Agent tools (file, shell, delegation, orchestration) |
| `macaca-proto` | Shared types and config structs |
| `macaca-persist` | Persistence layer with redb |
| `macaca-app` | App registry and runtime |
| `macaca-driver-claude-code` | Claude Code CLI integration |
| `macaca-memory` | Agent memory systems |
| `macaca-gateway` | Telegram/Discord bot integration |


---

## Prerequisites

### For Backend
- **Rust**: version 1.75 or higher

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

- **Cargo**: Comes with Rust installation

- **Claude Code CLI** (optional, for coding tasks)
```bash
npm install -g @anthropic-ai/claude-code
```

- **Milvus** (optional, for vector memory)
```bash
docker run -d --name milvus-standalone \
  -p 19530:19530 \
  -p 9091:9091 \
  -v /path/to/milvus:/var/lib/milvus \
  milvusdb/milvus:latest
```

- **NATS Server** (optional, for IPC - auto-started by default)
```bash
nats-server -p 4222
```

### For Frontend
- **Node.js**: version 18 or higher
```bash
# Using nvm (recommended)
nvm install 18
nvm use 18
```

- **npm or pnpm**: Comes with Node.js

## Installation

### 1. Clone the Repository

```bash
git clone <repository-url>
cd macaca
```

### 2. Backend Installation

```bash
# Build the Rust workspace
cargo build --release

# Or for development with faster builds
cargo build
```

The build will compile all 21 crates in the workspace.

### 3. Frontend Installation

```bash
cd frontend
npm install
# or
pnpm install
```

---

## Configuration

### LLM API Keys

Macaca supports multiple LLM providers. Configure your API keys in `config/default.toml` or as environment variables.

#### Supported Providers

| Provider | How to Get API Key |
|----------|-------------------|
| **OpenAI** | Visit https://platform.openai.com/api-keys |
| **Anthropic** | Visit https://console.anthropic.com/settings/keys |
| **DashScope (Alibaba)** | Visit https://dashscope.console.aliyun.com/apiKey |
| **DeepSeek** | Visit https://platform.deepseek.com/api_keys |
| **MiniMax** | Visit https://api.minimaxi.com/user-center/basic-information/interface-key |
| **Volces** | Contact Volces for API access |
| **OpenRouter** | Visit https://openrouter.ai/keys |

#### Setting API Keys

**Option 1: Environment Variables** (Recommended for security)

```bash
# OpenAI
export OPENAI_API_KEY="sk-..."

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."

# DashScope
export DASHSCOPE_API_KEY="sk-..."

# DeepSeek
export DEEPSEEK_API_KEY="sk-..."

# OpenRouter
export OPENROUTER_API_KEY="sk-or-..."

# Telegram Bot (optional)
export TELEGRAM_BOT_TOKEN="your-bot-token"

# Discord Bot (optional)
export DISCORD_BOT_TOKEN="your-bot-token"
```

**Option 2: Direct Configuration**

Edit `config/default.toml`:

```toml
[llm.providers.openai]
api_key = "sk-your-actual-key-here"
base_url = "https://api.openai.com/v1"

[llm.providers.anthropic]
api_key = "sk-ant-your-key-here"
base_url = "https://api.anthropic.com"
```

#### Setting Default Provider

Edit `config/default.toml`:

```toml
[llm]
default_provider = "openai"  # or "anthropic", "dashscope", etc.
```

### Other Configuration

See `config/default.toml` for additional settings:
- Kernel settings (max agents, timeouts)
- Memory configuration (TTL, vector store)
- IPC settings (NATS URL)
- Persistence settings (data directory)
- Observability (log level, tracing)
- Gateway settings (Telegram/Discord)

## Running the Application

### Starting the Backend

```bash
# From project root
cargo run --release --bin macaca-web

# Or use the binary directly
./target/release/macaca-web

# The API server starts on port 3001 by default
```

You should see:
```
Macaca OS API server: http://localhost:3001
```

### Starting the Frontend

In a new terminal:

```bash
cd frontend

# Development mode (with hot reload)
npm run dev

# Or production mode
npm run build
npm start
```

The frontend runs on http://localhost:3000

### Accessing the Application

1. Open your browser to http://localhost:3000
2. You'll see the Macaca dashboard with:
   - List of discovered applications
   - Agent status panels
   - Chat interface
   - Task board view

### Running with Docker (Optional)

```bash
# Build and run backend
docker build -t macaca-backend .
docker run -p 3001:3001 \
  -e OPENAI_API_KEY="sk-..." \
  -v $(pwd)/data:/app/data \
  macaca-backend

# Frontend
cd frontend
docker build -t macaca-frontend .
docker run -p 3000:3000 macaca-frontend
```

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

## Development

### Project Structure

```
macaca/
├── Cargo.toml              # Workspace manifest
├── config/
│   └── default.toml        # Main configuration
├── crates/
│   ├── macaca-web/         # HTTP API server
│   ├── macaca-kernel/      # Agent scheduling & execution
│   ├── macaca-runtime/     # Agentic loop
│   ├── macaca-task/        # TaskBoard, TodoStore, loops
│   ├── macaca-llm/         # LLM abstraction
│   ├── macaca-tools/       # Agent tools
│   ├── macaca-proto/       # Shared types
│   └── ...                 # (21 crates total)
├── examples/
│   └── apps/               # Example applications
│       └── fullstack-autodev/
│           ├── app.yaml    # App manifest
│           ├── agents/     # Agent definitions
│           ├── IDENTITY.md
│           ├── TOOLS.md
│           └── SOUL.md
├── docs/                   # Documentation
│   ├── SYSTEM_OVERVIEW.md
│   └── SYSTEM_AUDIT.md
├── frontend/               # Next.js frontend
│   ├── app/
│   ├── components/
│   └── lib/
└── src/                    # Legacy source files
```

### Creating a New Application

1. Create app directory in `examples/apps/your-app/`
2. Create `app.yaml` manifest:

```yaml
name: "your-app"
version: "1.0.0"
description: "Your application description"
agents:
  - name: "coordinator"
    persona: "IDENTITY.md"
    tools: "TOOLS.md"
    soul: "SOUL.md"
    capabilities:
      - name: "planning"
      - name: "delegation"
  - name: "worker"
    persona: "IDENTITY.md"
    tools: "TOOLS.md"
    soul: "SOUL.md"
    capabilities:
      - name: "coding"
      - name: "analysis"
```

3. Create agent definition files (`IDENTITY.md`, `TOOLS.md`, `SOUL.md`)
4. Restart the server or call `POST /api/apps/reload`

### Adding a Skill

Knowledge skills (from `SKILL.md`):

```markdown
# Skill Name

## Description
Brief description of what this skill does.

## Usage
How to use this skill.

## Example
Example of the skill in action.
```

Executable skill tools (from `skill.yaml` or code):

```yaml
name: "my-skill"
description: "My custom skill tool"
command: "python3 scripts/my-skill.py"
```

### Testing

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test '*'

# Run with output
cargo test -- --nocapture
```

## Troubleshooting

### Backend Won't Start

**Problem**: Server fails to start

**Solutions**:
1. Check Rust version: `rustc --version` (need 1.75+)
2. Ensure port 3001 is available: `lsof -i :3001`
3. Check configuration syntax: `cargo run -- 2>&1 | head`
4. Verify dependencies: `cargo check`

### API Key Errors

**Problem**: `api_key not found` or authentication errors

**Solutions**:
1. Set environment variables:
   ```bash
   export OPENAI_API_KEY="sk-..."
   ```
2. Or edit `config/default.toml` directly
3. Verify key is valid with the provider
4. Check provider base URL is correct

### Frontend Connection Refused

**Problem**: Frontend can't connect to backend

**Solutions**:
1. Ensure backend is running on port 3001
2. Check CORS configuration (should allow all origins in dev)
3. Verify API client base URL in `frontend/lib/api.ts`
4. Check browser console for network errors

### Agents Not Responding

**Problem**: Agents idle, no task execution

**Solutions**:
1. Check agent status: `GET /api/apps/{id}/agents`
2. Verify PlanLoop and WorkerLoop are running
3. Check LLM provider is configured correctly
4. Review logs in `./logs/` directory
5. Inspect task board: `GET /api/apps/{id}/todos`

### Database Errors

**Problem**: redb or persistence errors

**Solutions**:
1. Ensure data directory exists: `mkdir -p ./data`
2. Check disk space
3. Clear corrupted database: `rm ./data/persist/sessions.db`
4. Verify file permissions

### Memory/Vector Store Errors

**Problem**: Milvus connection errors

**Solutions**:
1. Ensure Milvus is running: `docker ps | grep milvus`
2. Check Milvus URL in config (default: `http://localhost:19530`)
3. Disable vector store if not needed:
   ```toml
   [memory.vector]
   backend = "none"
   ```
4. Verify embedding provider API key

### High Costs from LLM

**Problem**: Unexpected LLM usage costs

**Solutions**:
1. Enable cost tracking (already enabled by default)
2. Set max budget:
   ```toml
   [llm]
   max_budget_usd = 10.0
   ```
3. Use rate limiting: `rate_limit_rpm = 60`
4. Monitor cost tracker logs
5. Configure cheaper models as fallbacks

### Loop Detection Warnings

**Problem**: Agent stuck in repetitive loops

**Solutions**:
1. Check `docs/SYSTEM_AUDIT.md` for loop detection patterns
2. Review agent prompts for circular reasoning
3. Add context window limits in config
4. Enable and review audit logs

### Gateway Bot Issues

**Problem**: Telegram or Discord bots not responding

**Solutions**:
1. Verify bot token is set in environment variables
2. Check bot is added to the correct server/channel
3. Test bot permissions in Discord/Telegram
4. Review gateway logs in `./logs/`

## Architecture

### High-Level Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Frontend (Next.js)                    │
│                  http://localhost:3000                    │
└──────────────────────┬──────────────────────────────────────┘
                       │ HTTP + SSE
┌──────────────────────▼──────────────────────────────────────┐
│              Backend API (Axum, port 3001)               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              macaca-web crate                        │  │
│  └──────────────────────┬───────────────────────────────┘  │
│                       │                                   │
│  ┌──────────────────────▼──────────────────────────────┐  │
│  │           macaca-kernel & macaca-runtime            │  │
│  │         (Agent Scheduling & Execution)               │  │
│  └──────────────────────┬──────────────────────────────┘  │
└──────────────────────┬─┴──────────────────────────────────┘
                       │
        ┌──────────────┼──────────────┐
        │              │              │
┌───────▼─────┐  ┌──▼──────────┐ ┌─▼──────────────┐
│  macaca-llm  │  │macaca-task   │ │macaca-persist   │
│ (Providers)  │  │(TodoStore)   │ │  (redb DB)     │
└──────────────┘  └─────────────┘ └────────────────┘
```

### Key Components

1. **Kernel**: Manages agent lifecycle, scheduling, and execution
2. **Runtime**: Handles application registration and initialization
3. **Task System**: TodoStore with PlanLoop and WorkerLoop for task management
4. **LLM Layer**: Multi-provider abstraction with resilient wrapper
5. **Persistence**: Embedded database for sessions, todos, and events
6. **Tools**: Built-in (file, shell) + skill tools + Claude Code integration
7. **Gateways**: Telegram and Discord bot integrations

### Data Flow

1. User sends chat message via frontend
2. API server routes to chat orchestrator
3. Coordinator agent plans and delegates tasks via Fork-Join
4. Worker agents claim and execute tasks from TodoStore
5. Results streamed back via SSE
6. All events persisted to redb database

---

## Future Enhancements

### Short Term
- [ ] Enhanced task board UI with drag-and-drop prioritization
- [ ] WebSocket support for bidirectional real-time communication
- [ ] Additional LLM provider integrations (Claude, Cohere, etc.)
- [ ] Improved error recovery and retry mechanisms
- [ ] Docker Compose setup for easy deployment

### Medium Term
- [ ] Multi-tenancy support with user isolation
- [ ] Plugin system for custom tool development
- [ ] Advanced scheduling with dependencies between tasks
- [ ] Performance dashboard and metrics visualization
- [ ] Automated testing framework for agent workflows

### Long Term
- [ ] Distributed execution across multiple nodes
- [ ] Agent marketplace for sharing and discovering agents
- [ ] Natural language interface for system configuration
- [ ] Self-healing capabilities and automated optimization
- [ ] Integration with CI/CD pipelines for agent deployment

### Community Contributions

We welcome contributions! Areas of interest:
- New LLM provider integrations
- Custom skill implementations
- UI/UX improvements
- Performance optimizations
- Documentation enhancements
- Bug fixes and testing

See `docs/CONTRIBUTING.md` for contribution guidelines.

---

## License

MIT License - See LICENSE file for details

---

## Support & Community

- **Documentation**: See `docs/` directory for detailed architecture docs
- **Issues**: Report bugs and feature requests on GitHub
- **Discussions**: Join community discussions for help and ideas

---

**Built with ❤️ for the autonomous agent future**
