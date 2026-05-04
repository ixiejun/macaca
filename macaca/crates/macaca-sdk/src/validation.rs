//! Validation chain for SDK agent configuration.

use macaca_proto::{MacacaError, MacacaResult};

use crate::config::AgentConfig;

/// A single config validator in the SDK validation chain.
pub trait SdkValidator: Send + Sync {
    /// Validate a config or return a config error.
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()>;
}

/// Ordered validation chain for SDK config rules.
#[derive(Default)]
pub struct SdkValidationChain {
    validators: Vec<Box<dyn SdkValidator>>,
}

impl SdkValidationChain {
    /// Create an empty validation chain.
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    /// Add a validator.
    pub fn with_validator(mut self, validator: impl SdkValidator + 'static) -> Self {
        self.validators.push(Box::new(validator));
        self
    }

    /// Create the default validation chain.
    pub fn default_rules() -> Self {
        Self::new()
            .with_validator(NameValidator)
            .with_validator(PermissionLevelValidator)
            .with_validator(CapabilityNameValidator)
            .with_validator(TemperatureValidator)
    }

    /// Run validators in order.
    pub fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        for validator in &self.validators {
            validator.validate(config)?;
        }
        Ok(())
    }
}

struct NameValidator;

impl SdkValidator for NameValidator {
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        if config.name.trim().is_empty() {
            return Err(MacacaError::Config(
                "Agent config 'name' must not be empty".into(),
            ));
        }
        Ok(())
    }
}

struct PermissionLevelValidator;

impl SdkValidator for PermissionLevelValidator {
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        match config.permission_level.as_str() {
            "system" | "user" => Ok(()),
            other => Err(MacacaError::Config(format!(
                "Invalid permission_level '{other}'. Must be 'system' or 'user'"
            ))),
        }
    }
}

struct CapabilityNameValidator;

impl SdkValidator for CapabilityNameValidator {
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        for cap in &config.capabilities {
            if cap.name.trim().is_empty() {
                return Err(MacacaError::Config(
                    "Capability name must not be empty".into(),
                ));
            }
        }
        Ok(())
    }
}

struct TemperatureValidator;

impl SdkValidator for TemperatureValidator {
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        if let Some(temp) = config.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(MacacaError::Config(format!(
                    "Temperature {temp} out of range [0.0, 2.0]"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CapabilityDef;

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

    #[test]
    fn default_chain_rejects_invalid_permission() {
        let mut config = valid_config();
        config.permission_level = "admin".into();
        let err = SdkValidationChain::default_rules()
            .validate(&config)
            .unwrap_err();
        assert!(err.to_string().contains("permission_level"));
    }

    #[test]
    fn default_chain_rejects_empty_capability_name() {
        let mut config = valid_config();
        config.capabilities[0].name.clear();
        let err = SdkValidationChain::default_rules()
            .validate(&config)
            .unwrap_err();
        assert!(err.to_string().contains("Capability name"));
    }

    #[test]
    fn default_chain_rejects_temperature_out_of_range() {
        let mut config = valid_config();
        config.temperature = Some(3.0);
        let err = SdkValidationChain::default_rules()
            .validate(&config)
            .unwrap_err();
        assert!(err.to_string().contains("Temperature"));
    }
}
