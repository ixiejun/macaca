//! System service descriptor adapter for the LLM layer.
//!
//! This adapter is a descriptor skeleton only.  It lets Route C service
//! discovery see the LLM capability surface without changing how existing LLM
//! providers are selected or called.

use crate::service_contract::{LLM_CHAT_COMMAND, LLM_MODEL_SELECTION_COMMAND, LLM_SERVICE_ID};
use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, ServiceCapability, ServiceDescriptor,
    ServiceHealth, ServiceScope, ServiceType, TraceSchemaRef,
};

/// Build the provider-neutral descriptor for the LLM system service.
pub fn llm_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(LLM_SERVICE_ID),
        ServiceType::new("llm"),
        TraceSchemaRef::new("trace.system_service.llm.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            CapabilityId::new("capability.llm.chat"),
            "Runs model-backed conversational completion through the LLM service boundary.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.llm.model_selection"),
            "Resolves provider-neutral model routing metadata without exposing provider secrets.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.llm.snapshot"),
            "Emits sanitized LLM service inventory and health snapshots.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![ServiceScope::Global, ServiceScope::Application("*".into())];
    descriptor.required_permissions = vec!["llm.call".into()];
    descriptor
        .metadata
        .insert("command.chat".into(), LLM_CHAT_COMMAND.into());
    descriptor.metadata.insert(
        "command.model_selection".into(),
        LLM_MODEL_SELECTION_COMMAND.into(),
    );
    descriptor.cleanup_policy = CleanupPolicy::None;
    descriptor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_descriptor_exports_contract_shape() {
        let descriptor = llm_service_descriptor();
        assert_eq!(descriptor.service_type.as_str(), "llm");
        assert_eq!(descriptor.capabilities.len(), 3);
        assert!(descriptor.required_permissions.contains(&"llm.call".into()));
    }
}
