
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

