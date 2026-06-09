//! App loader — parses app manifests and loads agent configurations.

use std::path::Path;

use macaca_proto::{AgentConfig, AgentSkillsConfig, CapabilityDef, MacacaError, MacacaResult};

use crate::model::{AgentSource, AppLayer, AppManifest, InlineAgentConfig};
use crate::ui_runtime::validate_ui_runtime_config;

/// Loads and validates application manifests.
pub struct AppLoader;

impl AppLoader {
    /// Load an app manifest from a YAML file.
    pub fn load_manifest(path: impl AsRef<Path>) -> MacacaResult<AppManifest> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Self::parse_manifest_yaml(&content)
    }

    /// Parse an app manifest from a YAML string.
    pub fn parse_manifest_yaml(yaml: &str) -> MacacaResult<AppManifest> {
        let mut manifest: AppManifest =
            serde_yaml::from_str(yaml).map_err(|e| MacacaError::Config(e.to_string()))?;
        // Ensure deterministic ID based on app name (survives restarts)
        manifest.id = macaca_proto::ApplicationId::from_name(&manifest.name);
        Self::validate_manifest(&manifest)?;
        Ok(manifest)
    }

    /// Validate a parsed manifest.
    pub fn validate_manifest(manifest: &AppManifest) -> MacacaResult<()> {
        if manifest.name.trim().is_empty() {
            return Err(MacacaError::Config(
                "App manifest 'name' must not be empty".into(),
            ));
        }
        validate_ui_runtime_config(manifest.ui.as_ref())?;

        Ok(())
    }

    /// Resolve agent sources into [`AgentConfig`] values.
    ///
    /// - `FilePath` sources are loaded from disk relative to `base_dir`.
    /// - `Inline` sources are converted directly.
    /// - L1 native apps return an empty vec because those agents are
    ///   registered programmatically by the native application.
    /// - L2 WASM and L3 declarative apps both use manifest-declared agents.
    ///   The WASM guest still owns flexible orchestration, while Macaca
    ///   registers these app-scoped agent identities so host imports such as
    ///   `macaca:agent/delegate` can route without falling back to global
    ///   workers.
    pub fn resolve_agent_configs(
        manifest: &AppManifest,
        base_dir: impl AsRef<Path>,
    ) -> MacacaResult<Vec<AgentConfig>> {
        match manifest.layer {
            AppLayer::L1Native => Ok(vec![]),
            AppLayer::L2Wasm | AppLayer::L3Declarative => {
                let base = base_dir.as_ref();
                let mut configs = Vec::new();
                for source in &manifest.agents {
                    match source {
                        AgentSource::FilePath(rel_path) => {
                            let full_path = base.join(rel_path);
                            let config = AgentConfig::from_file(&full_path)?;
                            configs.push(config);
                        }
                        AgentSource::Inline(inline) => {
                            configs.push(inline_to_agent_config(inline));
                        }
                    }
                }
                Ok(configs)
            }
        }
    }
}

/// Convert an [`InlineAgentConfig`] into an [`AgentConfig`].
fn inline_to_agent_config(inline: &InlineAgentConfig) -> AgentConfig {
    AgentConfig {
        name: inline.name.clone(),
        capabilities: inline
            .capabilities
            .iter()
            .map(|c| CapabilityDef {
                name: c.name.clone(),
                description: c.description.clone(),
            })
            .collect(),
        permission_level: inline.permission_level.clone(),
        allowed_tools: inline.allowed_tools.clone(),
        allowed_paths: vec![],
        network_access: false,
        prompt_template: inline.prompt_template.clone(),
        model: inline.model.clone(),
        max_tokens: inline.max_tokens,
        temperature: inline.temperature,
        persona_dir: None,
        skills: inline
            .skills
            .as_ref()
            .map(|skills| AgentSkillsConfig {
                allow: skills.allow.clone(),
                deny: skills.deny.clone(),
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_manifest() {
        let yaml = r#"
name: test-app
version: "1.0.0"
layer: L3Declarative
agents:
  - name: helper
    prompt_template: "You help."
    capabilities:
      - name: assist
"#;
        let manifest = AppLoader::parse_manifest_yaml(yaml).unwrap();
        assert_eq!(manifest.name, "test-app");
        assert_eq!(manifest.layer, AppLayer::L3Declarative);
    }

    #[test]
    fn empty_name_fails() {
        let yaml = r#"
name: ""
layer: L3Declarative
"#;
        let err = AppLoader::parse_manifest_yaml(yaml).unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn l2_wasm_manifest_is_admitted() {
        let yaml = r#"
name: wasm-app
layer: L2Wasm
"#;
        let manifest = AppLoader::parse_manifest_yaml(yaml).unwrap();
        assert_eq!(manifest.layer, AppLayer::L2Wasm);
    }

    #[test]
    fn parse_manifest_with_ui_runtime_block() {
        let yaml = r#"
name: ui-app
layer: L2Wasm
ui:
  runtime: web_bundle
  framework: react
  entry: dist/ui/index.html
  assets:
    - dist/ui/assets/**
  sandbox:
    isolation: iframe
    csp: strict
    network: declared
  bridge:
    required:
      - service.call
      - trace.emit
    optional:
      - session.read
  theme:
    mode: app_owned
"#;
        let manifest = AppLoader::parse_manifest_yaml(yaml).unwrap();
        let ui = manifest.ui.expect("ui runtime declaration must parse");
        assert_eq!(ui.runtime, crate::ui_runtime::AppUiRuntimeKind::WebBundle);
        assert!(ui.bridge.declares("service.call"));
    }

    #[test]
    fn parse_manifest_rejects_escaping_ui_entry() {
        let yaml = r#"
name: ui-app
layer: L2Wasm
ui:
  runtime: web_bundle
  entry: ../escape.html
"#;
        let error = AppLoader::parse_manifest_yaml(yaml).unwrap_err();
        assert!(error.to_string().contains("package"));
    }

    #[test]
    fn resolve_l1_returns_empty() {
        let manifest = AppManifest {
            id: macaca_proto::ApplicationId::new(),
            name: "native".into(),
            description: None,
            version: "0.1.0".into(),
            layer: AppLayer::L1Native,
            ui_type: None,
            agents: vec![],
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
            service_contract: None,
            execution_profile: None,
            workbench: None,
            autonomy: None,
            ui: None,
            execution_control: None,
        };
        let configs = AppLoader::resolve_agent_configs(&manifest, ".").unwrap();
        assert!(configs.is_empty());
    }

    #[test]
    fn resolve_l2_without_manifest_agents_returns_empty() {
        let manifest = AppManifest {
            id: macaca_proto::ApplicationId::new(),
            name: "wasm".into(),
            description: None,
            version: "0.1.0".into(),
            layer: AppLayer::L2Wasm,
            ui_type: None,
            agents: vec![],
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
            service_contract: None,
            execution_profile: None,
            workbench: None,
            autonomy: None,
            ui: None,
            execution_control: None,
        };
        let configs = AppLoader::resolve_agent_configs(&manifest, ".").unwrap();
        assert!(configs.is_empty());
    }

    #[test]
    fn resolve_l2_inline_agents_for_wasm_orchestration() {
        let manifest = AppManifest {
            id: macaca_proto::ApplicationId::new(),
            name: "wasm-agent-app".into(),
            description: None,
            version: "0.1.0".into(),
            layer: AppLayer::L2Wasm,
            ui_type: None,
            agents: vec![AgentSource::Inline(InlineAgentConfig {
                name: "technical_analyst".into(),
                capabilities: vec![crate::model::CapabilityRef {
                    name: "technical_signal_analysis".into(),
                    description: "Analyze market evidence for a WASM app.".into(),
                }],
                prompt_template: "Use only provided evidence.".into(),
                model: "mock".into(),
                permission_level: "user".into(),
                allowed_tools: vec![],
                max_tokens: None,
                temperature: None,
                skills: None,
                context_engine: None,
            })],
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
            service_contract: None,
            execution_profile: None,
            workbench: None,
            autonomy: None,
            ui: None,
            execution_control: None,
        };
        let configs = AppLoader::resolve_agent_configs(&manifest, ".").unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].name, "technical_analyst");
        assert_eq!(configs[0].capabilities[0].name, "technical_signal_analysis");
    }

    #[test]
    fn resolve_inline_agents() {
        let manifest = AppManifest {
            id: macaca_proto::ApplicationId::new(),
            name: "inline-app".into(),
            description: None,
            version: "0.1.0".into(),
            layer: AppLayer::L3Declarative,
            ui_type: None,
            agents: vec![AgentSource::Inline(InlineAgentConfig {
                name: "inline-agent".into(),
                capabilities: vec![crate::model::CapabilityRef {
                    name: "cap1".into(),
                    description: "desc".into(),
                }],
                prompt_template: "Hello".into(),
                model: "gpt-4".into(),
                permission_level: "user".into(),
                allowed_tools: vec![],
                max_tokens: None,
                temperature: None,
                skills: None,
                context_engine: None,
            })],
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
            service_contract: None,
            execution_profile: None,
            workbench: None,
            autonomy: None,
            ui: None,
            execution_control: None,
        };
        let configs = AppLoader::resolve_agent_configs(&manifest, ".").unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].name, "inline-agent");
    }

    #[test]
    fn resolve_file_agents() {
        let dir = std::env::temp_dir().join("macaca_app_loader_test");
        std::fs::create_dir_all(&dir).unwrap();
        let agent_file = dir.join("agent.yaml");
        std::fs::write(
            &agent_file,
            r#"
name: file-loaded-agent
prompt_template: "From file"
capabilities:
  - name: fc
"#,
        )
        .unwrap();

        let manifest = AppManifest {
            id: macaca_proto::ApplicationId::new(),
            name: "file-app".into(),
            description: None,
            version: "0.1.0".into(),
            layer: AppLayer::L3Declarative,
            ui_type: None,
            agents: vec![AgentSource::FilePath("agent.yaml".into())],
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
            service_contract: None,
            execution_profile: None,
            workbench: None,
            autonomy: None,
            ui: None,
            execution_control: None,
        };
        let configs = AppLoader::resolve_agent_configs(&manifest, &dir).unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].name, "file-loaded-agent");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_manifest_with_service_contract_block() {
        let yaml = r#"
name: service-declared-app
layer: L2Wasm
service_contract:
  use_packs: ["pack.finance.v1"]
  required_services: ["service.market_data"]
  optional_services: ["service.news_digest"]
  service_policy_overrides:
    service.market_data:
      timeout_ms: 5000
      max_retries: 1
"#;
        let manifest = AppLoader::parse_manifest_yaml(yaml).unwrap();
        let contract = manifest
            .service_contract
            .expect("service contract must parse");
        assert_eq!(contract.use_packs, vec!["pack.finance.v1"]);
        assert_eq!(contract.required_services, vec!["service.market_data"]);
        assert_eq!(contract.optional_services, vec!["service.news_digest"]);
        assert_eq!(
            contract
                .service_policy_overrides
                .get("service.market_data")
                .and_then(|policy| policy.timeout_ms),
            Some(5000)
        );
    }
}
