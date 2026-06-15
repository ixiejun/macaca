//! System service descriptor adapter for the LLM layer.
//!
//! The canonical descriptor lives in `macaca-proto`. This module re-exports it
//! for provider-internal paths without forcing SDK callers to depend on the LLM
//! provider crate.

pub use macaca_proto::llm_service_descriptor;

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::LLM_CONTINUATION_VALIDATE_COMMAND;

    #[test]
    fn llm_descriptor_exports_contract_shape() {
        let descriptor = llm_service_descriptor();
        assert_eq!(descriptor.service_type.as_str(), "llm");
        assert_eq!(descriptor.capabilities.len(), 4);
        assert!(descriptor.required_permissions.contains(&"llm.call".into()));
        assert_eq!(
            descriptor.metadata.get("command.continuation_validate"),
            Some(&LLM_CONTINUATION_VALIDATE_COMMAND.to_string())
        );
    }
}
