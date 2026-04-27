//! Declarative compatibility registry.
//!
//! Maps skill install specs (`install.package` / `install.bins`) to MCP
//! server definitions when a skill does not provide an explicit
//! `mcpServers:` block. This replaces the previous `if
//! command.contains("playwright")` / `"@playwright/mcp"` hardcoding in
//! runtime source.
//!
//! The bundled mappings live in `resources/compat_mappings.toml` and are
//! embedded at build time via `include_str!`. Hosts can supply an override
//! file whose `id` replaces any bundled entry with the same id.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use macaca_framework::mcp::{McpSessionMode, McpTransportConfig};
use macaca_skill::SkillSnapshotEntry;
use serde::Deserialize;

use crate::mcp_runtime::{
    apply_concurrency_isolation, ConcurrencyIsolationPolicy, McpDefinitionSource,
    McpLifecycleScope, McpServerDefinition,
};

const BUNDLED_COMPAT_TOML: &str = include_str!("../resources/compat_mappings.toml");

#[derive(Debug, Clone, Deserialize, Default)]
struct CompatFile {
    #[serde(default)]
    compat: Vec<CompatEntry>,
}

/// A single compatibility mapping loaded from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct CompatEntry {
    pub id: String,
    #[serde(default)]
    pub match_packages: Vec<String>,
    #[serde(default)]
    pub match_bins: Vec<String>,
    pub server: CompatServer,
    #[serde(default)]
    pub concurrency_isolation: Option<CompatConcurrencyIsolation>,
}

/// Server template embedded in a compatibility entry.
#[derive(Debug, Clone, Deserialize)]
pub struct CompatServer {
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

/// Declarative concurrency-isolation policy (e.g. ensure `--isolated`).
#[derive(Debug, Clone, Deserialize)]
pub struct CompatConcurrencyIsolation {
    #[serde(default)]
    pub command_match: Vec<String>,
    #[serde(default)]
    pub required_args: Vec<String>,
    #[serde(default)]
    pub skip_if_any_arg_prefix: Vec<String>,
}

impl CompatConcurrencyIsolation {
    pub fn policy(&self) -> ConcurrencyIsolationPolicy {
        ConcurrencyIsolationPolicy {
            required_args: self.required_args.clone(),
            skip_if_any_arg_prefix: self.skip_if_any_arg_prefix.clone(),
        }
    }

    pub fn matches_command(&self, command: &str) -> bool {
        self.command_match
            .iter()
            .any(|pattern| command.contains(pattern))
    }
}

/// In-memory compatibility registry.
#[derive(Debug, Clone, Default)]
pub struct CompatRegistry {
    entries: Vec<CompatEntry>,
}

impl CompatRegistry {
    /// Registry built from the bundled TOML shipped with this crate.
    pub fn bundled() -> Self {
        let file: CompatFile =
            toml::from_str(BUNDLED_COMPAT_TOML).expect("bundled compat_mappings.toml is valid");
        Self {
            entries: file.compat,
        }
    }

    /// Registry built from an in-memory TOML string (useful for tests / hosts
    /// that embed configuration in their own config tree).
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let file: CompatFile = toml::from_str(text).map_err(|e| e.to_string())?;
        Ok(Self {
            entries: file.compat,
        })
    }

    /// Registry that starts from bundled defaults and layers an optional
    /// override file (entries with matching `id` replace bundled entries).
    pub fn load_with_override(override_path: Option<PathBuf>) -> Self {
        let mut base = Self::bundled();
        if let Some(path) = override_path {
            if let Ok(text) = std::fs::read_to_string(&path) {
                if let Ok(file) = toml::from_str::<CompatFile>(&text) {
                    for entry in file.compat {
                        base.entries.retain(|existing| existing.id != entry.id);
                        base.entries.push(entry);
                    }
                }
            }
        }
        base
    }

    pub fn entries(&self) -> &[CompatEntry] {
        &self.entries
    }

    /// Find the first compat entry whose `match_packages` / `match_bins`
    /// intersect the skill's install list.
    pub fn resolve_for_skill(&self, skill: &SkillSnapshotEntry) -> Option<&CompatEntry> {
        self.entries.iter().find(|entry| entry.matches_skill(skill))
    }

    /// Find a declarative concurrency-isolation policy that applies to
    /// `command` — used when a skill supplies its own `mcpServers:` entry
    /// whose command still needs the declarative safety net.
    pub fn policy_for_command(&self, command: &str) -> Option<ConcurrencyIsolationPolicy> {
        self.entries
            .iter()
            .filter_map(|entry| entry.concurrency_isolation.as_ref())
            .find(|iso| iso.matches_command(command))
            .map(CompatConcurrencyIsolation::policy)
    }
}

impl CompatEntry {
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

    /// Build a fully-resolved MCP server definition from this mapping.
    ///
    /// Returns `None` for non-stdio transports, which this compat layer does
    /// not (yet) generate.
    pub fn to_definition(&self, id: String) -> Option<McpServerDefinition> {
        if !self.server.transport.eq_ignore_ascii_case("stdio") {
            return None;
        }
        let policy = self
            .concurrency_isolation
            .as_ref()
            .map(CompatConcurrencyIsolation::policy);
        let args = if let Some(ref p) = policy {
            apply_concurrency_isolation(p, self.server.args.clone())
        } else {
            self.server.args.clone()
        };
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

/// Process-wide default compat registry — bundled mappings only.
pub fn default_registry() -> &'static CompatRegistry {
    static REGISTRY: OnceLock<CompatRegistry> = OnceLock::new();
    REGISTRY.get_or_init(CompatRegistry::bundled)
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
        let registry = CompatRegistry::bundled();
        let entry = registry
            .resolve_for_skill(&entry_with_pkg("@playwright/mcp"))
            .expect("playwright mapping must be present in bundled toml");
        assert_eq!(entry.id, "playwright");
        assert!(entry.concurrency_isolation.is_some());
    }

    #[test]
    fn playwright_definition_injects_isolated_flag() {
        let registry = CompatRegistry::bundled();
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
        let registry = CompatRegistry::bundled();
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
        let tmp = std::env::temp_dir().join("macaca-compat-override-test.toml");
        std::fs::write(&tmp, override_toml).unwrap();
        let registry = CompatRegistry::load_with_override(Some(tmp.clone()));
        let entry = registry
            .resolve_for_skill(&entry_with_pkg("@playwright/mcp"))
            .unwrap();
        assert_eq!(entry.server.command, "overridden");
        let _ = std::fs::remove_file(&tmp);
    }
}
