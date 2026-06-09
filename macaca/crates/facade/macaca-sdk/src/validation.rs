//! SDK validation chain — thin adapter over the proto declarative config validators.
//!
//! Preserves the historical `SdkValidationChain` / `SdkValidator` names for SDK consumers
//! while delegating rule evaluation to the shared proto contract.

use macaca_proto::{AgentConfig, DeclarativeAgentConfigValidation, MacacaResult};

/// A single config validator in the SDK validation chain (alias of proto validator trait).
pub trait SdkValidator: Send + Sync {
    /// Validate a config or return a config error.
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()>;
}

/// Ordered validation chain for SDK config rules.
#[derive(Default)]
pub struct SdkValidationChain {
    inner: DeclarativeAgentConfigValidation,
}

impl SdkValidationChain {
    /// Create an empty validation chain.
    pub fn new() -> Self {
        Self {
            inner: DeclarativeAgentConfigValidation::new(),
        }
    }

    /// Add a validator.
    pub fn with_validator(mut self, validator: impl SdkValidator + 'static) -> Self {
        self.inner = self.inner.with_validator(SdkValidatorAdapter(validator));
        self
    }

    /// Create the default validation chain (proto foundation rules).
    pub fn default_rules() -> Self {
        Self {
            inner: DeclarativeAgentConfigValidation::default_rules(),
        }
    }

    /// Run validators in order.
    pub fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        self.inner.validate(config)
    }
}

/// Bridges SDK-named validators into the proto validation chain.
struct SdkValidatorAdapter<V>(V);

impl<V> macaca_proto::DeclarativeAgentConfigValidator for SdkValidatorAdapter<V>
where
    V: SdkValidator,
{
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        self.0.validate(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::CapabilityDef;

    fn valid_config() -> AgentConfig {
        AgentConfig {
            name: "valid".into(),
            capabilities: vec![CapabilityDef {
                name: "planning".into(),
                description: String::new(),
            }],
            permission_level: "user".into(),
            allowed_tools: vec![],
            allowed_paths: vec![],
            network_access: false,
            prompt_template: String::new(),
            model: "gpt-4".into(),
            max_tokens: None,
            temperature: Some(1.0),
            persona_dir: None,
            skills: None,
        }
    }

    #[test]
    fn default_chain_accepts_valid_config() {
        SdkValidationChain::default_rules()
            .validate(&valid_config())
            .unwrap();
    }

    #[test]
    fn default_chain_rejects_empty_name() {
        let mut config = valid_config();
        config.name = " ".into();
        let err = SdkValidationChain::default_rules()
            .validate(&config)
            .unwrap_err();
        assert!(err.to_string().contains("name"));
    }
}
