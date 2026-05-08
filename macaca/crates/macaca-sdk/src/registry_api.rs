//! Registration helpers for declarative agents.

use std::path::Path;

use macaca_kernel::Kernel;
use macaca_proto::{AgentId, MacacaResult};

use crate::config::AgentConfig;
use crate::facade::MacacaSdk;

/// Build a [`DeclarativeAgent`](crate::DeclarativeAgent) from an [`AgentConfig`]
/// and register it with the kernel.
#[deprecated(note = "use MacacaSdk::for_kernel(kernel).register_config(config) instead")]
pub async fn register_from_config(kernel: &Kernel, config: AgentConfig) -> MacacaResult<AgentId> {
    MacacaSdk::for_kernel(kernel).register_config(config).await
}

/// Load an agent config from a file and register the resulting agent with the kernel.
///
/// File format is detected by extension (`.yaml`/`.yml` or `.toml`).
#[deprecated(
    note = "use AgentConfig::from_file(path) plus MacacaSdk::for_kernel(kernel).register_config(config) instead"
)]
pub async fn register_from_file(kernel: &Kernel, path: impl AsRef<Path>) -> MacacaResult<AgentId> {
    let config = AgentConfig::from_file(path)?;
    MacacaSdk::for_kernel(kernel).register_config(config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use async_trait::async_trait;
    use macaca_kernel::{KernelBuilder, KernelProviderCompat};
    use macaca_llm::LlmProvider;
    use macaca_proto::config::KernelConfig;
    use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, MacacaResult as Res, TokenUsage};
    use macaca_tools::DefaultToolSet;

    struct MockLlm;

    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock"
        }
        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> Res<LlmResponse> {
            Ok(LlmResponse {
                content: "ok".into(),
                reasoning_content: None,
                model: "mock".into(),
                usage: TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    fn make_kernel() -> Kernel {
        let config = KernelConfig {
            max_agents: 16,
            heartbeat_interval_ms: 5000,
            agent_timeout_ms: 30000,
        };
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
        KernelBuilder::from_compat(
            config,
            KernelProviderCompat::new(llm, Box::new(DefaultToolSet::new())),
        )
        .build()
    }

    #[tokio::test]
    async fn register_from_config_succeeds() {
        let kernel = make_kernel();
        let config = AgentConfig::from_yaml(
            r#"
name: reg-test
capabilities:
  - name: test
prompt_template: "Hello"
"#,
        )
        .unwrap();

        let id = register_from_config(&kernel, config).await.unwrap();
        let agents = kernel.list_agents().await;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, id);
        assert_eq!(agents[0].name, "reg-test");
    }

    #[tokio::test]
    async fn register_from_file_yaml() {
        let dir = std::env::temp_dir().join("macaca_sdk_test");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("test-agent.yaml");
        std::fs::write(
            &file,
            r#"
name: file-agent
capabilities:
  - name: file_cap
    description: From file
prompt_template: "You load from a file."
"#,
        )
        .unwrap();

        let kernel = make_kernel();
        let id = register_from_file(&kernel, &file).await.unwrap();
        let agents = kernel.list_agents().await;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, id);
        assert_eq!(agents[0].name, "file-agent");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn register_from_file_toml() {
        let dir = std::env::temp_dir().join("macaca_sdk_test_toml");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("test-agent.toml");
        std::fs::write(
            &file,
            r#"
name = "toml-file-agent"
prompt_template = "From toml file."

[[capabilities]]
name = "toml_cap"
description = "From TOML file"
"#,
        )
        .unwrap();

        let kernel = make_kernel();
        let id = register_from_file(&kernel, &file).await.unwrap();
        let agents = kernel.list_agents().await;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, id);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn register_invalid_config_fails() {
        let kernel = make_kernel();
        let config = AgentConfig {
            name: String::new(),
            capabilities: vec![],
            permission_level: "user".into(),
            allowed_tools: vec![],
            allowed_paths: vec![],
            network_access: false,
            prompt_template: String::new(),
            model: "gpt-4".into(),
            max_tokens: None,
            temperature: None,
            persona_dir: None,
            skills: None,
        };
        let err = register_from_config(&kernel, config).await.unwrap_err();
        assert!(err.to_string().contains("name"));
    }
}
