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

