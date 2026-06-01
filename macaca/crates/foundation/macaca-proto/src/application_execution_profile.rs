//! Application execution profile declarations for application manifests.
//!
//! This module is manifest-facing protocol data.  It lets an application
//! declare which generic execution provider shape it can use without making the
//! OS branch on application names, workflows, or business domains.  Runtime
//! providers consume the declaration through service-owned validation and
//! descriptor construction; shells only see sanitized API results.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ApplicationExecutionControlKind, ApplicationExecutionHeartbeatPolicy,
    ApplicationExecutionProviderPreference, CapabilityId, MacacaError, MacacaResult,
};

/// Manifest-level execution profile for one application package.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationExecutionProfileDeclaration {
    /// Optional provider preference carried as data into start commands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_preference: Option<ApplicationExecutionProviderPreference>,
    /// Optional external backend declaration used by the external provider
    /// adapter.  The declaration is inert until admitted by Application
    /// Framework and registered by runtime-host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_backend: Option<ExternalApplicationBackendExecutionProfile>,
    /// Bounded operator/application metadata.  Raw secrets, provider payloads,
    /// prompts, manifests, credentials, and tokens must never be stored here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationExecutionProfileDeclaration {
    /// Validate the manifest profile before it crosses into runtime-host.
    pub fn validate(&self) -> MacacaResult<()> {
        if let Some(external_backend) = &self.external_backend {
            external_backend.validate()?;
        }
        Ok(())
    }
}

/// External application backend execution profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalApplicationBackendExecutionProfile {
    /// Stable provider id used for descriptor registration and audit.
    pub provider_id: String,
    /// Start endpoint or endpoint reference.  It is treated as opaque data here
    /// because concrete HTTP/local transports belong to runtime-host adapters.
    pub start_endpoint: String,
    /// Optional control endpoint or endpoint reference for later control
    /// delivery.  Missing values mean control forwarding is not yet available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_endpoint: Option<String>,
    /// Application execution protocol version expected by the backend.
    pub protocol_version: String,
    /// Gateway URL/ref the backend should call when reporting execution facts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_gateway_ref: Option<String>,
    /// Scoped callback identity reference.  This is a credential reference, not
    /// a credential value, and may safely appear in sanitized descriptors.
    pub callback_identity_ref: String,
    /// Controls this backend can receive through the generic control protocol.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_controls: Vec<ApplicationExecutionControlKind>,
    /// Heartbeat interval and timeout expected from the backend.
    pub heartbeat_policy: ApplicationExecutionHeartbeatPolicy,
    /// Start/control request timeout used by runtime-host transport adapters.
    pub request_timeout_ms: u64,
    /// Event schema version accepted for callback ingress.
    pub event_schema_version: String,
    /// Capability declarations exposed by this backend descriptor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capability_declarations: Vec<CapabilityId>,
    /// Generic resource/profile labels used by provider selection.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resource_profile: BTreeMap<String, String>,
}

impl ExternalApplicationBackendExecutionProfile {
    /// Validate provider-neutral shape before runtime-host registration.
    pub fn validate(&self) -> MacacaResult<()> {
        require_text(&self.provider_id, "external backend provider_id")?;
        require_text(&self.start_endpoint, "external backend start_endpoint")?;
        require_text(&self.protocol_version, "external backend protocol_version")?;
        require_text(
            &self.callback_identity_ref,
            "external backend callback_identity_ref",
        )?;
        require_text(
            &self.event_schema_version,
            "external backend event_schema_version",
        )?;
        if let Some(control_endpoint) = &self.control_endpoint {
            require_text(control_endpoint, "external backend control_endpoint")?;
        }
        if let Some(callback_gateway_ref) = &self.callback_gateway_ref {
            require_text(
                callback_gateway_ref,
                "external backend callback_gateway_ref",
            )?;
        }
        if self.heartbeat_policy.interval_ms == 0 || self.heartbeat_policy.timeout_ms == 0 {
            return Err(MacacaError::Config(
                "external backend heartbeat interval and timeout must be positive".into(),
            ));
        }
        if self.request_timeout_ms == 0 {
            return Err(MacacaError::Config(
                "external backend request_timeout_ms must be positive".into(),
            ));
        }
        if self.supported_controls.is_empty() {
            return Err(MacacaError::Config(
                "external backend supported_controls must not be empty".into(),
            ));
        }
        Ok(())
    }
}

fn require_text(value: &str, field: &str) -> MacacaResult<()> {
    if value.trim().is_empty() {
        return Err(MacacaError::Config(format!("{field} must not be empty")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ApplicationExecutionControlKind, ApplicationExecutionProviderKind,
        ApplicationExecutionProviderPreference,
    };

    fn valid_external_profile() -> ExternalApplicationBackendExecutionProfile {
        ExternalApplicationBackendExecutionProfile {
            provider_id: "provider.external.fixture".into(),
            start_endpoint: "https://backend.example.test/start".into(),
            control_endpoint: Some("https://backend.example.test/control".into()),
            protocol_version: "application-execution.v1".into(),
            callback_gateway_ref: Some("gateway/application-execution".into()),
            callback_identity_ref: "identity.external.fixture".into(),
            supported_controls: vec![ApplicationExecutionControlKind::Cancel],
            heartbeat_policy: ApplicationExecutionHeartbeatPolicy {
                interval_ms: 1_000,
                timeout_ms: 5_000,
                required: true,
            },
            request_timeout_ms: 3_000,
            event_schema_version: "application-execution.v1".into(),
            capability_declarations: vec![CapabilityId::new("capability.application_execution")],
            resource_profile: BTreeMap::new(),
        }
    }

    #[test]
    fn external_backend_profile_roundtrips_and_validates() {
        let declaration = ApplicationExecutionProfileDeclaration {
            provider_preference: Some(ApplicationExecutionProviderPreference {
                provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
                provider_id: Some("provider.external.fixture".into()),
            }),
            external_backend: Some(valid_external_profile()),
            metadata: BTreeMap::new(),
        };

        declaration.validate().unwrap();
        let encoded = serde_json::to_string(&declaration).unwrap();
        let decoded: ApplicationExecutionProfileDeclaration =
            serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, declaration);
    }

    #[test]
    fn external_backend_profile_rejects_incomplete_shape() {
        let mut profile = valid_external_profile();
        profile.callback_identity_ref.clear();

        let error = profile.validate().unwrap_err();

        assert!(error
            .to_string()
            .contains("external backend callback_identity_ref"));
    }
}
