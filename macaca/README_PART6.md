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

