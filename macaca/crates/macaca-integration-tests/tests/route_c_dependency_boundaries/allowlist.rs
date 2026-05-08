use super::gate::AllowlistEntry;

/// Returns the current Route C migration-debt snapshot used by the executable
/// dependency gate.
///
/// This table deliberately mirrors
/// `macaca/docs/route-c-serviceization-allowlist.md`. The Rust copy is the
/// deterministic execution input; the markdown copy is the human governance
/// memento. Adding a row here without the corresponding OpenSpec and document
/// update would hide architectural debt, so the gate diagnostics explicitly tell
/// maintainers to update both surfaces.
pub fn entries() -> Vec<AllowlistEntry> {
    vec![
        AllowlistEntry::new(
            "kernel-no-provider-deps",
            "macaca-kernel",
            "macaca-driver",
            "S6",
            "Driver Service facade",
        ),
        AllowlistEntry::new(
            "kernel-no-provider-deps",
            "macaca-kernel",
            "macaca-gateway",
            "S8",
            "Gateway Service facade",
        ),
        AllowlistEntry::new(
            "kernel-no-provider-deps",
            "macaca-kernel",
            "macaca-llm",
            "S5",
            "LLM Service facade",
        ),
        AllowlistEntry::new(
            "kernel-no-provider-deps",
            "macaca-kernel",
            "macaca-memory",
            "S5",
            "Memory/Context Service facade",
        ),
        AllowlistEntry::new(
            "kernel-no-provider-deps",
            "macaca-kernel",
            "macaca-persist",
            "S1/S2",
            "Persistence Service contract",
        ),
        AllowlistEntry::new(
            "kernel-no-provider-deps",
            "macaca-kernel",
            "macaca-skill",
            "S6",
            "Skill/MCP Service facade",
        ),
        AllowlistEntry::new(
            "kernel-no-provider-deps",
            "macaca-kernel",
            "macaca-task",
            "S4",
            "Task Service facade",
        ),
        AllowlistEntry::new(
            "kernel-no-provider-deps",
            "macaca-kernel",
            "macaca-tools",
            "S6",
            "Tool/Skill Service facade",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-cli",
            "macaca-gateway",
            "S8",
            "Gateway Service client",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-cli",
            "macaca-llm",
            "S5",
            "LLM Service client",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-cli",
            "macaca-tools",
            "S6",
            "Tool/Skill Service client",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-driver",
            "S6",
            "Driver Service client",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-llm",
            "S5",
            "LLM Service client",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-memory",
            "S5",
            "Memory/Context Service client",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-persist",
            "S1/S12",
            "Persistence Service client",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-skill",
            "S6",
            "Skill/MCP Service client",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-task",
            "S4",
            "Task Service client",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-tools",
            "S6",
            "Tool/Skill Service client",
        ),
        AllowlistEntry::new(
            "cli-no-web-internals",
            "macaca-cli",
            "macaca-web",
            "S12",
            "shared shell facade in macaca-sdk",
        ),
    ]
}
