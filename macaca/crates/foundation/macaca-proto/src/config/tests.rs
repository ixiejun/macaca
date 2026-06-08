//! Integration tests for [`super::MacacaConfig`] loading and builder serde shape.

use super::*;

#[test]
fn context_recall_is_disabled_by_default() {
    let cfg = MacacaConfig::default();
    assert!(!cfg.context.recall.expose_memory_tools);
    assert!(!cfg.context.recall.preflight_recall_enabled);
}

#[test]
fn default_config_is_valid() {
    let cfg = MacacaConfig::default();
    assert_eq!(cfg.kernel.max_agents, 16);
    assert_eq!(cfg.context.default_engine, "legacy");
    assert!(cfg.context.emit_reports);
    assert!(!cfg.context.agent_profile.enabled);
    assert_eq!(cfg.context.workspace_guides.entries.len(), 6);
    assert!(cfg.context.external_adapters.is_empty());
    assert_eq!(cfg.memory.embedding.model, "text-embedding-v4");
    assert_eq!(cfg.memory.vector.backend, "milvus");
    assert_eq!(cfg.memory.embedding.dimensions, 1024);
    assert!(cfg.memory.provider_runtime.providers.is_empty());
    assert!(cfg.gateway.telegram.unwrap().enabled);
}

#[test]
fn load_nonexistent_falls_back_to_default() {
    let cfg = MacacaConfig::load_default();
    assert_eq!(cfg.persist.engine, "redb");
}

#[test]
fn macaca_config_builder_matches_default_then_overrides() {
    let built = MacacaConfigBuilder::new()
        .workspace(WorkspaceConfig {
            root_dir: "/tmp/workspaces".into(),
        })
        .drivers(DriversConfig {
            directory: "custom-drivers".into(),
            auto_load: false,
        })
        .build();

    assert_eq!(
        built.kernel.max_agents,
        MacacaConfig::default().kernel.max_agents
    );
    assert_eq!(built.workspace.root_dir, "/tmp/workspaces");
    assert_eq!(built.drivers.directory, "custom-drivers");
    assert!(!built.drivers.auto_load);
}

#[test]
fn llm_provider_config_builder_matches_manual_construction() {
    let manual = LlmProviderConfig {
        api_key_plan: Some("PLAN_KEY".into()),
        api_key: "PAYGO_KEY".into(),
        base_url: "https://example.com/v1".into(),
        default_model: Some("gpt-test".into()),
    };

    let built = LlmProviderConfigBuilder::new("https://example.com/v1")
        .api_key_plan("PLAN_KEY")
        .api_key("PAYGO_KEY")
        .default_model("gpt-test")
        .build();

    assert_eq!(built.api_key_plan, manual.api_key_plan);
    assert_eq!(built.api_key, manual.api_key);
    assert_eq!(built.base_url, manual.base_url);
    assert_eq!(built.default_model, manual.default_model);
}

#[test]
fn macaca_config_builder_preserves_serde_shape() {
    let json = serde_json::to_value(MacacaConfigBuilder::new().build()).unwrap();
    assert!(json.get("kernel").is_some());
    assert!(json.get("llm").is_some());
    assert!(json.get("drivers").is_some());
}

#[test]
fn load_external_context_adapter_config_from_toml() {
    let raw = r#"
            [kernel]
            max_agents = 1
            heartbeat_interval_ms = 1000
            agent_timeout_ms = 1000

            [llm]
            default_provider = "test"
            max_tokens_per_request = 1
            rate_limit_rpm = 1

            [llm.providers.test]
            api_key = "KEY"
            base_url = "https://example.com"

            [memory]
            session_ttl_seconds = 1
            file_store_path = "./tmp"
            auto_retrieve_on = "task_start"

            [memory.vector]
            backend = "milvus"
            milvus_url = "http://localhost:19530"
            collection_name = "agent_memory"

            [memory.embedding]
            provider = "dashscope"
            model = "text-embedding-v4"
            api_key = "KEY"
            dimensions = 1024
            base_url = "https://example.com"

            [memory.compression]
            enabled = false
            threshold_entries = 1
            strategy = "none"

            [ipc]
            nats_url = "nats://localhost:4222"
            nats_auto_start = false
            reconnect_max_attempts = 1
            reconnect_delay_ms = 1

            [persist]
            engine = "redb"
            data_dir = "./data"
            snapshot_interval_seconds = 1

            [gateway]
            enabled = false

            [observability]
            log_level = "info"
            tracing_enabled = false
            otlp_endpoint = ""

            [[context.external_adapters]]
            id = "workspace-sidecar"
            transport = "http_json"

            [context.external_adapters.http_json]
            url = "http://127.0.0.1:8787/assemble"
            headers = { AUTHORIZATION = "EXTERNAL_CONTEXT_TOKEN" }

            [context.external_adapters.fallback]
            fallback_engine_id = "legacy"
            empty_external_contribution = true
        "#;

    let cfg: MacacaConfig = toml::from_str(raw).unwrap();
    assert_eq!(cfg.context.external_adapters.len(), 1);
    let adapter = &cfg.context.external_adapters[0];
    assert_eq!(adapter.id, "workspace-sidecar");
    assert_eq!(
        adapter.transport,
        ContextExternalAdapterTransportKind::HttpJson
    );
    assert_eq!(
        adapter.http_json.as_ref().unwrap().url,
        "http://127.0.0.1:8787/assemble"
    );
    assert_eq!(
        adapter
            .http_json
            .as_ref()
            .unwrap()
            .headers
            .get("AUTHORIZATION")
            .unwrap(),
        "EXTERNAL_CONTEXT_TOKEN"
    );
}
