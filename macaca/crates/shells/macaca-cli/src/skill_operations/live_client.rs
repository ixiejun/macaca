//! HTTP Adapter over the public Web Skill operations API.
//!
//! The adapter intentionally knows only route transport details: app id, base
//! URL, request method, and bounded JSON payloads.  It does not decode Skill
//! service DTOs or make lifecycle decisions, preserving Web/service ownership
//! of curation semantics while still allowing CLI to prove a live runtime path.

use macaca_proto::{MacacaError, MacacaResult};
use serde_json::Value;
use tracing::info;

use super::support::{http_error, normalize_api_base, url_segment};

/// Small HTTP client bound to one application id and API base.
///
/// **Adapter pattern**: isolates REST transport from command handlers so SDK and
/// live paths share the same handler modules.
#[derive(Debug, Clone)]
pub(crate) struct LiveSkillOperationsClient {
    api_base: String,
    pub(crate) app_id: String,
    http: reqwest::Client,
}

impl LiveSkillOperationsClient {
    /// Construct a client after validating the API base URL scheme.
    pub(crate) fn new(api_base: String, app_id: String) -> MacacaResult<Self> {
        let api_base = normalize_api_base(&api_base)?;
        Ok(Self {
            api_base,
            app_id,
            http: reqwest::Client::new(),
        })
    }

    /// Issue a GET against the Skill operations facade for the bound application.
    pub(crate) async fn get(&self, path: &str) -> MacacaResult<Value> {
        let url = self.url(path);
        info!(
            app_id = %self.app_id,
            url = %url,
            method = "GET",
            "CLI requesting live Skill operations state from Web API facade"
        );
        let response = self.http.get(url).send().await.map_err(http_error)?;
        decode_live_response(response).await
    }

    /// Issue a POST with a bounded operator payload against the facade.
    pub(crate) async fn post(
        &self,
        path: &str,
        payload: LiveSkillOperatorPayload,
    ) -> MacacaResult<Value> {
        let url = self.url(path);
        info!(
            app_id = %self.app_id,
            url = %url,
            method = "POST",
            evidence_count = payload.evidence_ids.len(),
            policy_decision_count = payload.policy_decision_refs.len(),
            approval_count = payload.approval_refs.len(),
            "CLI forwarding live Skill operation to Web API facade"
        );
        let response = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(http_error)?;
        decode_live_response(response).await
    }

    /// Build the fully qualified REST URL for a relative operations path.
    pub(crate) fn url(&self, path: &str) -> String {
        format!(
            "{}/api/apps/{}/skills/operations{}",
            self.api_base,
            url_segment(&self.app_id),
            path
        )
    }
}

/// JSON body for live Skill operator mutations forwarded through the Web facade.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct LiveSkillOperatorPayload {
    pub(crate) reason: String,
    pub(crate) rationale: String,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) policy_decision_refs: Vec<String>,
    pub(crate) approval_refs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rollback_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stale_after_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) narrow_use_threshold: Option<u64>,
    pub(crate) source: String,
    pub(crate) source_scope: String,
}

impl LiveSkillOperatorPayload {
    /// Attach curation threshold knobs for run/apply endpoints.
    pub(crate) fn with_curation_thresholds(
        mut self,
        stale_after_days: i64,
        narrow_use_threshold: u64,
    ) -> Self {
        self.stale_after_days = Some(stale_after_days);
        self.narrow_use_threshold = Some(narrow_use_threshold);
        self
    }

    /// Attach a rollback memento ref for rollback endpoints.
    pub(crate) fn with_rollback_ref(mut self, rollback_ref: String) -> Self {
        self.rollback_ref = Some(rollback_ref);
        self
    }
}

/// Decode a live HTTP response, surfacing server error messages when present.
async fn decode_live_response(response: reqwest::Response) -> MacacaResult<Value> {
    let status = response.status();
    let data = response.json::<Value>().await.map_err(http_error)?;
    if status.is_success() {
        return Ok(data);
    }
    let reason = data
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or("skill live operations request failed");
    Err(MacacaError::Config(format!(
        "skill live operations API returned HTTP {status}: {reason}"
    )))
}
