use serde::{Deserialize, Serialize};
use serde_json::Value;

use macaca_proto::{MacacaError, MacacaResult, MemoryEntry, MemoryId};

use crate::core::facade::{
    MemoryDeleteRequest, MemoryFacade, MemoryGetRequest, MemorySearchRequest, MemoryWriteRequest,
};
use crate::core::provider::{MemoryProvider, MemoryProviderDescriptor};
use crate::core::status::MemoryCapabilitySet;
use crate::core::status::MemoryStatusReport;

use super::config::MemoryProviderConfig;
use super::resilience::{redact_text, MemoryProviderResilience};

/// Remote request envelope for the `macaca-memory-v1` protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProviderRemoteEnvelope<T> {
    pub scope: Value,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub timeout_ms: u64,
    pub payload: T,
}

/// Remote response envelope for the `macaca-memory-v1` protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProviderRemoteResult {
    pub healthy: bool,
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub payload: Option<Value>,
}

/// A small transport adapter that talks to an external memory provider over HTTP.
pub struct RemoteMemoryProvider {
    provider_id: String,
    base_url: String,
    client: reqwest::Client,
    resilience: MemoryProviderResilience,
    display_name: String,
}

impl RemoteMemoryProvider {
    /// Build a remote provider from configuration.
    pub fn new(config: &MemoryProviderConfig) -> MacacaResult<Self> {
        let endpoint = config.endpoint.as_ref().ok_or_else(|| {
            MacacaError::Memory(format!(
                "remote provider {} is missing endpoint configuration",
                config.id
            ))
        })?;
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| MacacaError::Memory(format!("remote client build failed: {e}")))?;
        Ok(Self {
            provider_id: config.id.clone(),
            base_url: endpoint.url.trim_end_matches('/').to_string(),
            client,
            resilience: MemoryProviderResilience::new(&config.resilience),
            display_name: config
                .display_name
                .clone()
                .unwrap_or_else(|| config.id.clone()),
        })
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    fn scope_value(&self, request_scope: &crate::core::scope::MemoryScope) -> Value {
        serde_json::to_value(request_scope).unwrap_or_else(|_| Value::Null)
    }

    fn diagnostics_redaction_markers(&self) -> Vec<String> {
        vec![self.base_url.clone()]
    }

    async fn post_json<T>(
        &self,
        operation: &'static str,
        path: &str,
        envelope: &MemoryProviderRemoteEnvelope<T>,
    ) -> MacacaResult<MemoryProviderRemoteResult>
    where
        T: Serialize + Clone,
    {
        let payload = serde_json::to_vec(envelope)
            .map_err(|e| MacacaError::Memory(format!("{operation} serialize failed: {e}")))?;
        let url = self.endpoint(path);
        let client = self.client.clone();
        let request = envelope.clone();
        let redaction = self.diagnostics_redaction_markers();
        self.resilience
            .execute(operation, payload.len(), move || {
                let client = client.clone();
                let url = url.clone();
                let request = request.clone();
                let redaction = redaction.clone();
                async move {
                    let resp = client.post(url).json(&request).send().await.map_err(|e| {
                        MacacaError::Memory(format!("{operation} request failed: {e}"))
                    })?;
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_default();
                        return Err(MacacaError::Memory(format!(
                            "{operation} {status}: {}",
                            redact_text(&text, &redaction)
                        )));
                    }
                    resp.json::<MemoryProviderRemoteResult>()
                        .await
                        .map_err(|e| MacacaError::Memory(format!("{operation} parse failed: {e}")))
                }
            })
            .await
    }

    fn parse_entries(payload: Option<Value>) -> MacacaResult<Vec<MemoryEntry>> {
        match payload {
            Some(Value::Array(items)) => items
                .into_iter()
                .map(|value| {
                    serde_json::from_value(value)
                        .map_err(|e| MacacaError::Memory(format!("remote entry parse failed: {e}")))
                })
                .collect(),
            Some(Value::Object(map)) => {
                if let Some(Value::Array(items)) = map.get("entries") {
                    items
                        .iter()
                        .cloned()
                        .map(|value| {
                            serde_json::from_value(value).map_err(|e| {
                                MacacaError::Memory(format!("remote entry parse failed: {e}"))
                            })
                        })
                        .collect()
                } else {
                    Ok(Vec::new())
                }
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Probe the remote provider status endpoint.
    pub async fn probe_status(&self) -> MacacaResult<MemoryProviderRemoteResult> {
        let url = self.endpoint("memory/v1/status");
        let client = self.client.clone();
        let redaction = self.diagnostics_redaction_markers();
        self.resilience
            .execute("memory.status", 0, move || {
                let client = client.clone();
                let url = url.clone();
                let redaction = redaction.clone();
                async move {
                    let resp = client.get(url).send().await.map_err(|e| {
                        MacacaError::Memory(format!("memory.status request failed: {e}"))
                    })?;
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_default();
                        return Err(MacacaError::Memory(format!(
                            "memory.status {status}: {}",
                            redact_text(&text, &redaction)
                        )));
                    }
                    resp.json::<MemoryProviderRemoteResult>()
                        .await
                        .map_err(|e| {
                            MacacaError::Memory(format!("memory.status parse failed: {e}"))
                        })
                }
            })
            .await
    }

    /// Send an informational event to the remote provider.
    pub async fn record_event(&self, event: Value) -> MacacaResult<MemoryProviderRemoteResult> {
        let envelope = MemoryProviderRemoteEnvelope {
            scope: Value::Null,
            trace_id: None,
            timeout_ms: self.resilience.timeout().as_millis() as u64,
            payload: event,
        };
        self.post_json("memory.events", "memory/v1/events", &envelope)
            .await
    }

    fn parse_entry(payload: Option<Value>) -> MacacaResult<Option<MemoryEntry>> {
        match payload {
            Some(value) => serde_json::from_value(value)
                .map(Some)
                .map_err(|e| MacacaError::Memory(format!("remote entry parse failed: {e}"))),
            None => Ok(None),
        }
    }
}

#[async_trait::async_trait]
impl MemoryFacade for RemoteMemoryProvider {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        let envelope = MemoryProviderRemoteEnvelope {
            scope: self.scope_value(&request.scope),
            trace_id: request
                .metadata
                .get("trace_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            timeout_ms: self.resilience.timeout().as_millis() as u64,
            payload: request,
        };
        let response = self
            .post_json("memory.write", "memory/v1/write", &envelope)
            .await?;
        let entry = Self::parse_entry(response.payload)?
            .ok_or_else(|| MacacaError::Memory("remote write returned no memory entry".into()))?;
        Ok(entry.id)
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        let envelope = MemoryProviderRemoteEnvelope {
            scope: self.scope_value(&request.scope),
            trace_id: None,
            timeout_ms: self.resilience.timeout().as_millis() as u64,
            payload: request,
        };
        let response = self
            .post_json("memory.search", "memory/v1/search", &envelope)
            .await?;
        Self::parse_entries(response.payload)
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        let envelope = MemoryProviderRemoteEnvelope {
            scope: self.scope_value(&request.scope),
            trace_id: None,
            timeout_ms: self.resilience.timeout().as_millis() as u64,
            payload: request,
        };
        let response = self
            .post_json("memory.get", "memory/v1/get", &envelope)
            .await?;
        Self::parse_entry(response.payload)
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        let envelope = MemoryProviderRemoteEnvelope {
            scope: self.scope_value(&request.scope),
            trace_id: None,
            timeout_ms: self.resilience.timeout().as_millis() as u64,
            payload: request,
        };
        let _ = self
            .post_json("memory.delete", "memory/v1/delete", &envelope)
            .await?;
        Ok(())
    }

    fn status(&self) -> MemoryStatusReport {
        MemoryStatusReport::healthy(
            self.provider_id.clone(),
            MemoryCapabilitySet {
                store: true,
                search: true,
                prompt: true,
                lifecycle: true,
                flush: true,
                artifact: false,
                governance: true,
                knowledge: true,
            },
        )
    }
}

impl MemoryProvider for RemoteMemoryProvider {
    fn descriptor(&self) -> MemoryProviderDescriptor {
        MemoryProviderDescriptor::new(
            self.provider_id.clone(),
            self.display_name.clone(),
            MemoryCapabilitySet {
                store: true,
                search: true,
                prompt: true,
                lifecycle: true,
                flush: true,
                artifact: false,
                governance: true,
                knowledge: true,
            },
        )
    }
}
