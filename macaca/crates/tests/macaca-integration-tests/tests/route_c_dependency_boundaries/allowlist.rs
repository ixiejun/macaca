use super::gate::AllowlistEntry;

/// Returns the current Route C migration-debt snapshot used by the executable
/// dependency gate.
///
/// This table deliberately mirrors
/// `macaca/docs/macaca-os-serviceization-allowlist.md`. The Rust copy is the
/// deterministic execution input; the markdown copy is the human governance
/// memento. Adding a row here without the corresponding OpenSpec and document
/// update would hide architectural debt, so the gate diagnostics explicitly tell
/// maintainers to update both surfaces.
pub fn entries() -> Vec<AllowlistEntry> {
    vec![
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-driver",
            "Web thin shell",
            "macaca-web/Cargo.toml direct dependency used by AppState and framework toolkit migration paths",
            "S6",
            "Driver Service client",
            "cargo metadata --no-deps --format-version 1",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-llm",
            "Web thin shell",
            "macaca-web/Cargo.toml direct dependency used by service-client migration paths",
            "S5",
            "LLM Service client",
            "cargo metadata --no-deps --format-version 1",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-memory",
            "Web thin shell",
            "macaca-web/Cargo.toml direct dependency used by memory/context compatibility paths",
            "S5",
            "Memory/Context Service client",
            "cargo metadata --no-deps --format-version 1",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-persist",
            "Web thin shell",
            "macaca-web/Cargo.toml direct dependency used by shell persistence migration paths",
            "S1/S12",
            "Persistence Service client",
            "cargo metadata --no-deps --format-version 1",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-skill",
            "Web thin shell",
            "macaca-web/Cargo.toml direct dependency used by skill/MCP compatibility paths",
            "S6",
            "Skill/MCP Service client",
            "cargo metadata --no-deps --format-version 1",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-task",
            "Web thin shell",
            "macaca-web/Cargo.toml direct dependency used by Web-owned session loop migration paths",
            "S4",
            "Task Service client",
            "cargo metadata --no-deps --format-version 1",
        ),
        AllowlistEntry::new(
            "presentation-no-provider-construction-hub",
            "macaca-web",
            "macaca-tools",
            "Web thin shell",
            "macaca-web/Cargo.toml direct dependency used by toolkit migration paths",
            "S6",
            "Tool/Skill Service client",
            "cargo metadata --no-deps --format-version 1",
        ),
    ]
}
