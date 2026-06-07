//! Declarative skill-to-MCP mapping registry (Registry pattern).
//!
//! Maps skill install specs (`install.package` / `install.bins`) to MCP server
//! definitions when a skill does not provide an explicit `mcpServers:` block.
//! This replaces product-name `if command.contains(...)` branches in runtime
//! control flow with auditable, host-overridable configuration.
//!
//! Bundled mappings ship in `resources/compat_mappings.toml` (embedded via
//! `include_str!`). Hosts may override entries through
//! `$MACACA_HOME/compat_mappings.toml` or an explicit override path passed to
//! [`SkillMcpMappingRegistry::load_with_override`]. The on-disk TOML table key
//! remains `[[compat]]` so existing operator overrides keep working.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use macaca_framework::mcp::{McpSessionMode, McpTransportConfig};
use macaca_skill::SkillSnapshotEntry;
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::mcp_runtime::{
    apply_concurrency_isolation, ConcurrencyIsolationPolicy, McpDefinitionSource,
    McpLifecycleScope, McpServerDefinition,
};

/// Embedded bundled mapping document compiled into the runtime-host binary.
const BUNDLED_SKILL_MCP_MAPPINGS_TOML: &str =
    include_str!("../resources/compat_mappings.toml");

/// Root TOML document for skill MCP mapping files.
///
/// The serde field name `compat` is retained for backward compatibility with
/// existing operator override files; it is not an OS "compat layer" module name.
#[derive(Debug, Clone, Deserialize, Default)]
struct SkillMcpMappingFile {
    #[serde(default)]
    compat: Vec<SkillMcpMappingEntry>,
}

/// One declarative mapping from skill install metadata to an MCP server template.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillMcpMappingEntry {
    pub id: String,
    #[serde(default)]
    pub match_packages: Vec<String>,
    #[serde(default)]
    pub match_bins: Vec<String>,
    pub server: SkillMcpMappingServer,
    #[serde(default)]
    pub concurrency_isolation: Option<SkillMcpConcurrencyIsolation>,
}

/// MCP server template embedded in a mapping entry.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillMcpMappingServer {
    pub transport: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub lifecycle: McpLifecycleScope,
    #[serde(default = "default_session_mode")]
    pub session_mode: McpSessionMode,
    #[serde(default)]
    pub tool_prefix: Option<String>,
    #[serde(default)]
    pub required_bins: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

fn default_session_mode() -> McpSessionMode {
    McpSessionMode::Stateful
}

/// Declarative concurrency-isolation policy (for example ensure `--isolated`).
#[derive(Debug, Clone, Deserialize)]
pub struct SkillMcpConcurrencyIsolation {
    #[serde(default)]
    pub command_match: Vec<String>,
    #[serde(default)]
    pub required_args: Vec<String>,
    #[serde(default)]
    pub skip_if_any_arg_prefix: Vec<String>,
}

impl SkillMcpConcurrencyIsolation {
    /// Materialize the runtime policy object used by MCP definition assembly.
    pub fn policy(&self) -> ConcurrencyIsolationPolicy {
        ConcurrencyIsolationPolicy {
            required_args: self.required_args.clone(),
            skip_if_any_arg_prefix: self.skip_if_any_arg_prefix.clone(),
        }
    }

    /// Returns true when `command` matches any configured substring pattern.
    pub fn matches_command(&self, command: &str) -> bool {
        self.command_match
            .iter()
            .any(|pattern| command.contains(pattern))
    }
}

/// In-memory registry of skill install spec → MCP server mappings.
#[derive(Debug, Clone, Default)]
pub struct SkillMcpMappingRegistry {
    entries: Vec<SkillMcpMappingEntry>,
}

impl SkillMcpMappingRegistry {
    /// Build the registry from the bundled TOML shipped with this crate.
    pub fn bundled() -> Self {
        let file: SkillMcpMappingFile = toml::from_str(BUNDLED_SKILL_MCP_MAPPINGS_TOML)
            .expect("bundled skill MCP mapping TOML is valid");
        let entry_count = file.compat.len();
        info!(
            entry_count,
            "loaded bundled skill MCP mapping registry"
        );
        Self {
            entries: file.compat,
        }
    }

    /// Build a registry from an in-memory TOML string (tests / embedded host config).
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let file: SkillMcpMappingFile = toml::from_str(text).map_err(|e| e.to_string())?;
        debug!(
            entry_count = file.compat.len(),
            "parsed skill MCP mapping registry from TOML"
        );
        Ok(Self {
            entries: file.compat,
        })
    }

    /// Start from bundled defaults and layer an optional override file.
    ///
    /// Override entries with matching `id` replace bundled entries in place.
    pub fn load_with_override(override_path: Option<PathBuf>) -> Self {
        let mut base = Self::bundled();
        if let Some(path) = override_path {
            match std::fs::read_to_string(&path) {
                Ok(text) => match toml::from_str::<SkillMcpMappingFile>(&text) {
                    Ok(file) => {
                        let override_count = file.compat.len();
                        for entry in file.compat {
                            base.entries.retain(|existing| existing.id != entry.id);
                            debug!(mapping_id = %entry.id, "applied skill MCP mapping override");
                            base.entries.push(entry);
                        }
                        info!(
                            override_path = %path.display(),
                            override_count,
                            total_entries = base.entries.len(),
                            "merged skill MCP mapping override file"
                        );
                    }
                    Err(error) => {
                        warn!(
                            override_path = %path.display(),
                            %error,
                            "ignored invalid skill MCP mapping override file"
                        );
                    }
                },
                Err(error) => {
                    warn!(
                        override_path = %path.display(),
                        %error,
                        "skill MCP mapping override file unreadable"
                    );
                }
            }
        }
        base
    }

    /// Expose the loaded mapping entries for inspection or iteration.
    pub fn entries(&self) -> &[SkillMcpMappingEntry] {
        &self.entries
    }

    /// Resolve the first mapping whose package/bin patterns match the skill install list.
    pub fn resolve_for_skill(&self, skill: &SkillSnapshotEntry) -> Option<&SkillMcpMappingEntry> {
        let resolved = self.entries.iter().find(|entry| entry.matches_skill(skill));
        if let Some(entry) = resolved {
            debug!(
                skill = %skill.name,
                mapping_id = %entry.id,
                "resolved skill MCP mapping from install metadata"
            );
        }
        resolved
    }

    /// Find a declarative concurrency-isolation policy for an MCP server command.
    pub fn policy_for_command(&self, command: &str) -> Option<ConcurrencyIsolationPolicy> {
        self.entries
            .iter()
            .filter_map(|entry| entry.concurrency_isolation.as_ref())
            .find(|iso| iso.matches_command(command))
            .map(SkillMcpConcurrencyIsolation::policy)
    }
}

impl SkillMcpMappingEntry {
    fn matches_skill(&self, skill: &SkillSnapshotEntry) -> bool {
        skill.install.iter().any(|install| {
            if let Some(pkg) = install.package.as_deref() {
                if self.match_packages.iter().any(|p| p == pkg) {
                    return true;
                }
            }
            install
                .bins
                .iter()
                .any(|bin| self.match_bins.iter().any(|b| b == bin))
        })
    }

    /// Build a fully-resolved MCP server definition from this mapping entry.
    ///
    /// Returns `None` for non-stdio transports, which this registry does not generate.
    pub fn to_definition(&self, id: String) -> Option<McpServerDefinition> {
        if !self.server.transport.eq_ignore_ascii_case("stdio") {
            debug!(
                mapping_id = %self.id,
                transport = %self.server.transport,
                "skipping non-stdio skill MCP mapping"
            );
            return None;
        }
        let policy = self
            .concurrency_isolation
            .as_ref()
            .map(SkillMcpConcurrencyIsolation::policy);
        let args = if let Some(ref p) = policy {
            apply_concurrency_isolation(p, self.server.args.clone())
        } else {
            self.server.args.clone()
        };
        debug!(
            mapping_id = %self.id,
            server_id = %id,
            command = %self.server.command,
            "materialized MCP server definition from skill mapping"
        );
        Some(McpServerDefinition {
            id,
            transport: McpTransportConfig::Stdio {
                command: self.server.command.clone(),
                args,
                env: BTreeMap::new(),
                cwd: None,
            },
            lifecycle: self.server.lifecycle.clone(),
            session_mode: self.server.session_mode,
            tool_prefix: self.server.tool_prefix.clone(),
            required_bins: self.server.required_bins.clone(),
            enabled: self.server.enabled,
            source: McpDefinitionSource::Compatibility,
            concurrency_isolation: policy,
        })
    }
}

/// Process-wide default registry containing bundled mappings only.
pub fn default_skill_mcp_mapping_registry() -> &'static SkillMcpMappingRegistry {
    static REGISTRY: OnceLock<SkillMcpMappingRegistry> = OnceLock::new();
    REGISTRY.get_or_init(SkillMcpMappingRegistry::bundled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_skill::{SkillInstallSpec, SkillSourceScope};
    use std::path::PathBuf;

    fn entry_with_pkg(pkg: &str) -> SkillSnapshotEntry {
        SkillSnapshotEntry {
            name: "skill".into(),
            description: "d".into(),
            source_location: PathBuf::from("/tmp/SKILL.md"),
            source_base_dir: PathBuf::from("/tmp"),
            location: PathBuf::from("/tmp/SKILL.md"),
            base_dir: PathBuf::from("/tmp"),
            source: "t".into(),
            source_scope: SkillSourceScope::MacacaCentral,
            primary_env: None,
            required_env: Vec::new(),
            install: vec![SkillInstallSpec {
                kind: "npm".into(),
                package: Some(pkg.into()),
                bins: Vec::new(),
                ..Default::default()
            }],
            mcp_servers: Vec::new(),
        }
    }

    #[test]
    fn bundled_registry_contains_playwright_mapping() {
        let registry = SkillMcpMappingRegistry::bundled();
        let entry = registry
            .resolve_for_skill(&entry_with_pkg("@playwright/mcp"))
            .expect("playwright mapping must be present in bundled toml");
        assert_eq!(entry.id, "playwright");
        assert!(entry.concurrency_isolation.is_some());
    }

    #[test]
    fn playwright_definition_injects_isolated_flag() {
        let registry = SkillMcpMappingRegistry::bundled();
        let entry = registry
            .resolve_for_skill(&entry_with_pkg("@playwright/mcp"))
            .unwrap();
        let definition = entry.to_definition("test".into()).unwrap();
        match definition.transport {
            McpTransportConfig::Stdio { args, .. } => {
                assert!(args.iter().any(|a| a == "--isolated"));
            }
            _ => panic!("expected stdio"),
        }
        assert!(definition.concurrency_isolation.is_some());
    }

    #[test]
    fn policy_for_command_matches_by_substring() {
        let registry = SkillMcpMappingRegistry::bundled();
        assert!(registry.policy_for_command("playwright-mcp").is_some());
        assert!(registry.policy_for_command("unrelated-bin").is_none());
    }

    #[test]
    fn override_replaces_bundled_entry_by_id() {
        let override_toml = r#"
[[compat]]
id = "playwright"
match_packages = ["@playwright/mcp"]
match_bins = []

[compat.server]
transport = "stdio"
command = "overridden"
args = []
required_bins = ["overridden"]

[compat.concurrency_isolation]
command_match = ["overridden"]
required_args = ["--iso2"]
skip_if_any_arg_prefix = []
"#;
        let tmp = std::env::temp_dir().join("macaca-skill-mcp-mapping-override-test.toml");
        std::fs::write(&tmp, override_toml).unwrap();
        let registry = SkillMcpMappingRegistry::load_with_override(Some(tmp.clone()));
        let entry = registry
            .resolve_for_skill(&entry_with_pkg("@playwright/mcp"))
            .unwrap();
        assert_eq!(entry.server.command, "overridden");
        let _ = std::fs::remove_file(&tmp);
    }
}
