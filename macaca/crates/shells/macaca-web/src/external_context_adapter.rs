use std::sync::Arc;

use async_trait::async_trait;
use macaca_sdk::context::{
    ContextAdapterSafetyPolicy, ContextAssembleInput, ContextAssembleResult, ContextEngine,
    ContextEngineRegistry, ContextFallbackPolicy, ExternalAdapterContextEngine,
    ExternalContextAdapter, ExternalContextAdapterInfo,
};
use macaca_proto::config::{
    ContextConfig, ContextExternalAdapterConfig, ContextExternalAdapterHttpJsonConfig,
    ContextExternalAdapterTransportKind,
};
use macaca_proto::{MacacaError, MacacaResult};

use crate::state::ExternalAdapterRuntimeInstallation;

/// Startup product of config-driven external adapter installation.
///
/// The startup path needs two artifacts:
/// - a runtime registry overlay containing selectable `ContextEngine`s
/// - an operator-facing inventory snapshot used by `/api/context/provider-runtime`
///
/// Returning both together keeps installation deterministic and lets the caller wire state without
/// re-deriving inventory rows from engine metadata after the fact.
pub struct ConfiguredExternalContextAdapters {
    pub registry: ContextEngineRegistry,
    pub installations: Vec<ExternalAdapterRuntimeInstallation>,
}

/// Build external adapter engines declared in `context.external_adapters`.
///
/// This is Phase 9's config-driven installation seam: transports are instantiated at startup,
/// wrapped behind `ExternalAdapterContextEngine`, and overlaid into the neutral engine registry.
pub fn install_external_adapters_from_config(
    config: &ContextConfig,
) -> MacacaResult<ConfiguredExternalContextAdapters> {
    let mut registry = ContextEngineRegistry::new();
    let mut installations = Vec::new();
    for adapter in config
        .external_adapters
        .iter()
        .filter(|adapter| adapter.enabled)
    {
        let (engine, installation) = build_external_adapter(adapter)?;
        registry = registry.register(engine);
        installations.push(installation);
    }
    Ok(ConfiguredExternalContextAdapters {
        registry,
        installations,
    })
}

fn build_external_adapter(
    config: &ContextExternalAdapterConfig,
) -> MacacaResult<(Arc<dyn ContextEngine>, ExternalAdapterRuntimeInstallation)> {
    let id = config.id.trim();
    if id.is_empty() {
        return Err(MacacaError::Config(
            "context.external_adapters[].id must not be empty".into(),
        ));
    }

    let transport = match config.transport {
        ContextExternalAdapterTransportKind::HttpJson => {
            let http = config.http_json.as_ref().ok_or_else(|| {
                MacacaError::Config(format!(
                    "context.external_adapters['{id}'] requires http_json settings"
                ))
            })?;
            let adapter = HttpJsonExternalContextAdapter::from_config(config, http)?;
            (
                "http_json".to_string(),
                "context.external_adapters".to_string(),
                Arc::new(adapter) as Arc<dyn ExternalContextAdapter>,
            )
        }
    };

    let safety = ContextAdapterSafetyPolicy {
        timeout_ms: config.safety.timeout_ms,
        max_payload_bytes: config.safety.max_payload_bytes,
        require_schema_validation: config.safety.require_schema_validation,
        require_budget_validation: config.safety.require_budget_validation,
        circuit_breaker_failures: config.safety.circuit_breaker_failures,
    };
    let fallback = ContextFallbackPolicy {
        fallback_engine_id: config.fallback.fallback_engine_id.clone(),
        empty_external_contribution: config.fallback.empty_external_contribution,
    };
    let engine = Arc::new(
        ExternalAdapterContextEngine::new(
            Arc::clone(&transport.2),
            safety.clone(),
            fallback.clone(),
        )
        .with_fallback_registry(ContextEngineRegistry::with_builtins()),
    ) as Arc<dyn ContextEngine>;
    let engine_info = engine.info();
    let installation = ExternalAdapterRuntimeInstallation {
        engine: engine_info,
        transport: transport.0,
        installation_source: transport.1,
        runtime_state: "installed".into(),
        default_safety_policy: safety,
        default_fallback_policy: fallback,
        circuit_breaker_runtime_state: "not_implemented".into(),
        last_sync_epoch_ms: 0,
    };
    Ok((engine, installation))
}

/// Simple JSON-over-HTTP adapter transport.
///
/// The request body is the raw `ContextAssembleInput`, and the response body must be a serialized
/// `ContextAssembleResult`. This stays deliberately neutral so any sidecar or remote runtime can
/// participate without teaching the core OS about app-specific protocols.
struct HttpJsonExternalContextAdapter {
    info: ExternalContextAdapterInfo,
    endpoint_url: String,
    headers: Vec<(String, String)>,
    client: reqwest::Client,
}

impl HttpJsonExternalContextAdapter {
    fn from_config(
        config: &ContextExternalAdapterConfig,
        http: &ContextExternalAdapterHttpJsonConfig,
    ) -> MacacaResult<Self> {
        let endpoint_url = http.url.trim();
        if endpoint_url.is_empty() {
            return Err(MacacaError::Config(format!(
                "context.external_adapters['{}'].http_json.url must not be empty",
                config.id
            )));
        }

        let headers = http
            .headers
            .iter()
            .map(|(name, value)| {
                resolve_header_value(value).map(|resolved| (name.clone(), resolved))
            })
            .collect::<MacacaResult<Vec<_>>>()?;

        Ok(Self {
            info: ExternalContextAdapterInfo {
                id: config.id.clone(),
                version: config
                    .version
                    .clone()
                    .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
            },
            endpoint_url: endpoint_url.to_string(),
            headers,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(
                    config.safety.timeout_ms.max(1),
                ))
                .build()
                .map_err(|error| {
                    MacacaError::Config(format!(
                        "failed to build HTTP client for external adapter '{}': {error}",
                        config.id
                    ))
                })?,
        })
    }
}

#[async_trait]
impl ExternalContextAdapter for HttpJsonExternalContextAdapter {
    fn info(&self) -> ExternalContextAdapterInfo {
        self.info.clone()
    }

    async fn assemble(&self, input: ContextAssembleInput) -> MacacaResult<ContextAssembleResult> {
        let mut request = self.client.post(&self.endpoint_url).json(&input);
        for (name, value) in &self.headers {
            request = request.header(name, value);
        }
        let response = request.send().await.map_err(|error| {
            MacacaError::Agent(format!(
                "external context adapter '{}' request failed: {error}",
                self.info.id
            ))
        })?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(MacacaError::Agent(format!(
                "external context adapter '{}' returned HTTP {}: {}",
                self.info.id, status, body
            )));
        }
        response
            .json::<ContextAssembleResult>()
            .await
            .map_err(|error| {
                MacacaError::Serialization(format!(
                    "external context adapter '{}' returned invalid JSON: {error}",
                    self.info.id
                ))
            })
    }
}

/// Resolve one header value using the same neutral convention as LLM and MCP config.
///
/// Empty strings remain empty, literal strings are used verbatim, and all-caps identifiers are
/// treated as environment variable names so operators can keep secrets out of the TOML file.
fn resolve_header_value(raw: &str) -> MacacaResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let looks_like_env = trimmed
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch == '_' || ch.is_ascii_digit());
    if looks_like_env {
        std::env::var(trimmed)
            .map_err(|_| MacacaError::Config(format!("{trimmed} not set for external adapter")))
    } else {
        Ok(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::{routing::post, Json, Router};
    use macaca_sdk::context::{ContextEngineSelection, ContextReportBuilder, ContextRuntimeFacade};
    use macaca_proto::config::{
        ContextExternalAdapterFallbackConfig, ContextExternalAdapterSafetyConfig,
    };
    use macaca_proto::{LlmMessage, LlmOptions};

    async fn echo_context(Json(input): Json<ContextAssembleInput>) -> Json<ContextAssembleResult> {
        Json(ContextAssembleResult {
            messages: input.base_messages.clone(),
            options: input.options.clone(),
            options_patch: Default::default(),
            report: ContextReportBuilder::new("custom-http")
                .identity(
                    input.app_id,
                    input.session_id.clone(),
                    input.agent_name.clone(),
                    input.model.clone(),
                )
                .budget(input.budget)
                .engine("custom-http")
                .build(),
        })
    }

    fn test_adapter_config(url: String) -> ContextExternalAdapterConfig {
        ContextExternalAdapterConfig {
            id: "custom-http".into(),
            display_name: Some("Custom HTTP Adapter".into()),
            version: Some("1.2.3".into()),
            enabled: true,
            transport: ContextExternalAdapterTransportKind::HttpJson,
            http_json: Some(ContextExternalAdapterHttpJsonConfig {
                url,
                headers: std::collections::HashMap::new(),
            }),
            safety: ContextExternalAdapterSafetyConfig::default(),
            fallback: ContextExternalAdapterFallbackConfig::default(),
        }
    }

    #[test]
    fn install_external_adapters_registers_enabled_rows() {
        let mut config = ContextConfig::default();
        config.external_adapters = vec![test_adapter_config("http://127.0.0.1:1/assemble".into())];

        let installed = install_external_adapters_from_config(&config).unwrap();
        assert_eq!(installed.installations.len(), 1);
        assert_eq!(installed.installations[0].engine.id, "custom-http");
        assert_eq!(installed.installations[0].transport, "http_json");
        assert_eq!(
            installed.installations[0].installation_source,
            "context.external_adapters"
        );
        assert_eq!(installed.registry.list_engine_ids(), vec!["custom-http"]);
    }

    #[tokio::test]
    async fn http_json_external_adapter_can_be_selected_via_installed_registry() {
        let app = Router::new().route("/assemble", post(echo_context));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let mut config = ContextConfig::default();
        config.external_adapters = vec![test_adapter_config(format!(
            "http://127.0.0.1:{}/assemble",
            addr.port()
        ))];

        let installed = install_external_adapters_from_config(&config).unwrap();
        let facade = ContextRuntimeFacade::new(
            installed.registry,
            ContextEngineSelection {
                engine_id: "custom-http".into(),
                fallback_engine_id: "legacy".into(),
            },
        );
        let result = facade
            .assemble(ContextAssembleInput::legacy(
                "agent",
                "model",
                vec![LlmMessage::user("hello")],
                LlmOptions::default(),
            ))
            .await
            .unwrap();

        assert_eq!(result.report.engine_id, "custom-http");
        assert_eq!(result.report.requested_engine_id, "custom-http");
    }
}
