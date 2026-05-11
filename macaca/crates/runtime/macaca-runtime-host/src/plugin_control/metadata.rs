//! Metadata sanitization helpers for Plugin Control Plane.
//!
//! These helpers keep value filtering and package-declared requirement parsing
//! outside the service facade.  The control service can then focus on lifecycle
//! orchestration while this module owns audit-safe metadata shaping.

use std::collections::BTreeMap;

use macaca_proto::{PluginConfigRequirement, PluginSecretRequirement};

/// Remove keys that are likely to contain secret material before storing or
/// returning metadata through Web, CLI, SDK, diagnostics, or audit records.
pub fn sanitized_metadata(metadata: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    metadata
        .iter()
        .filter(|(key, _)| !key.to_ascii_lowercase().contains("secret"))
        .filter(|(key, _)| !key.to_ascii_lowercase().contains("token"))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// Parse comma-separated config keys declared by package metadata.
pub fn config_requirements(metadata: &BTreeMap<String, String>) -> Vec<PluginConfigRequirement> {
    metadata
        .get("required_config")
        .map(|value| {
            value
                .split(',')
                .filter_map(|key| {
                    let key = key.trim();
                    (!key.is_empty()).then(|| PluginConfigRequirement {
                        key: key.into(),
                        required: true,
                        present: false,
                        description: "Declared by plugin package metadata".into(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse comma-separated secret names without exposing secret values.
pub fn secret_requirements(metadata: &BTreeMap<String, String>) -> Vec<PluginSecretRequirement> {
    metadata
        .get("required_secrets")
        .map(|value| {
            value
                .split(',')
                .filter_map(|name| {
                    let name = name.trim();
                    (!name.is_empty()).then(|| PluginSecretRequirement {
                        name: name.into(),
                        required: true,
                        present: false,
                        source_hint: Some("env_or_secret_provider".into()),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}
