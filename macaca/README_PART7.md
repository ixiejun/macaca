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
