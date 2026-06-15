/// Resolves a provider id from a model reference.
pub trait ProviderResolver: Send + Sync {
    fn resolve(&self, model: &str) -> Option<String>;
}

/// Descriptor matcher used by model-route policy rows.
///
/// The LLM service treats model routing as data, not provider-specific control
/// flow. A descriptor can be loaded from built-in defaults today and from
/// service/package configuration later without changing resolver behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelRouteMatcher {
    /// Match provider/model references used by gateway aggregators.
    Slash,
    /// Match a model-family prefix.
    Prefix(String),
    /// Match a model-family prefix without treating case as routing semantics.
    CaseInsensitivePrefix(String),
}

impl ModelRouteMatcher {
    /// Return whether this descriptor matcher accepts the supplied model ref.
    fn matches(&self, model: &str) -> bool {
        match self {
            Self::Slash => model.contains('/'),
            Self::Prefix(prefix) => model.starts_with(prefix),
            Self::CaseInsensitivePrefix(prefix) => model
                .get(..prefix.len())
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix)),
        }
    }
}

/// Descriptor row that maps a model-family matcher to a provider id.
///
/// This is the Strategy-pattern data carried by [`PrefixProviderResolver`].
/// Keeping provider ids in descriptors makes the route table auditable and
/// replaceable while preserving the existing default behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRouteDescriptor {
    matcher: ModelRouteMatcher,
    provider: String,
}

impl ModelRouteDescriptor {
    /// Create a descriptor for provider/model references that contain `/`.
    pub fn slash(provider: impl Into<String>) -> Self {
        Self {
            matcher: ModelRouteMatcher::Slash,
            provider: provider.into(),
        }
    }

    /// Create a descriptor for a case-sensitive model prefix.
    pub fn prefix(prefix: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            matcher: ModelRouteMatcher::Prefix(prefix.into()),
            provider: provider.into(),
        }
    }

    /// Create a descriptor for a case-insensitive model prefix.
    pub fn case_insensitive_prefix(prefix: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            matcher: ModelRouteMatcher::CaseInsensitivePrefix(prefix.into()),
            provider: provider.into(),
        }
    }
}

/// Default model-route descriptors that preserve existing service behavior.
///
/// The values remain in one data table so audits can see every default route in
/// a single place. Future runtime-host or package configuration can replace
/// this descriptor list without changing resolver control flow.
pub fn default_model_route_descriptors() -> Vec<ModelRouteDescriptor> {
    vec![
        ModelRouteDescriptor::slash("openrouter"),
        ModelRouteDescriptor::prefix("gpt-", "openai"),
        ModelRouteDescriptor::prefix("o1", "openai"),
        ModelRouteDescriptor::prefix("o3", "openai"),
        ModelRouteDescriptor::prefix("claude-", "anthropic"),
        ModelRouteDescriptor::prefix("qwen", "dashscope"),
        ModelRouteDescriptor::prefix("deepseek-", "deepseek"),
        ModelRouteDescriptor::case_insensitive_prefix("minimax-", "minimax"),
    ]
}

/// Resolves model families through ordered descriptor rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixProviderResolver {
    descriptors: Vec<ModelRouteDescriptor>,
}

impl PrefixProviderResolver {
    /// Build a resolver from descriptor rows supplied by configuration.
    pub fn from_descriptors(descriptors: Vec<ModelRouteDescriptor>) -> Self {
        Self { descriptors }
    }

    /// Build the default resolver from descriptor data.
    pub fn built_in() -> Self {
        Self::from_descriptors(default_model_route_descriptors())
    }
}

impl ProviderResolver for PrefixProviderResolver {
    fn resolve(&self, model: &str) -> Option<String> {
        self.descriptors
            .iter()
            .find(|descriptor| descriptor.matcher.matches(model))
            .map(|descriptor| descriptor.provider.clone())
    }
}

/// Applies provider resolvers in order and falls back to the model reference.
pub struct ResolverChain {
    resolvers: Vec<Box<dyn ProviderResolver>>,
}

impl ResolverChain {
    pub fn new(resolvers: Vec<Box<dyn ProviderResolver>>) -> Self {
        Self { resolvers }
    }

    pub fn built_in() -> Self {
        Self::new(vec![Box::new(PrefixProviderResolver::built_in())])
    }

    /// Build a resolver chain from descriptor rows.
    ///
    /// This constructor is the stable extension point for descriptor-driven
    /// routing. Callers can swap the descriptor list without depending on
    /// concrete provider implementations or adding OS-layer provider branches.
    pub fn from_descriptors(descriptors: Vec<ModelRouteDescriptor>) -> Self {
        Self::new(vec![Box::new(PrefixProviderResolver::from_descriptors(
            descriptors,
        ))])
    }

    pub fn resolve_provider(&self, model: &str) -> String {
        self.resolvers
            .iter()
            .find_map(|resolver| resolver.resolve(model))
            .unwrap_or_else(|| model.to_string())
    }
}

impl Default for ResolverChain {
    fn default() -> Self {
        Self::built_in()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_routes_gpt_models_to_openai() {
        let resolver = ResolverChain::built_in();
        assert_eq!(resolver.resolve_provider("gpt-4o"), "openai");
        assert_eq!(resolver.resolve_provider("gpt-4-turbo"), "openai");
        assert_eq!(resolver.resolve_provider("o1-preview"), "openai");
        assert_eq!(resolver.resolve_provider("o3-mini"), "openai");
    }

    #[test]
    fn resolver_routes_claude_models_to_anthropic() {
        let resolver = ResolverChain::built_in();
        assert_eq!(
            resolver.resolve_provider("claude-3-5-sonnet-20241022"),
            "anthropic"
        );
    }

    #[test]
    fn resolver_routes_qwen_models_to_dashscope() {
        let resolver = ResolverChain::built_in();
        assert_eq!(resolver.resolve_provider("qwen-turbo"), "dashscope");
        assert_eq!(resolver.resolve_provider("qwen-max"), "dashscope");
        assert_eq!(resolver.resolve_provider("qwen-plus"), "dashscope");
        assert_eq!(resolver.resolve_provider("qwen3-max"), "dashscope");
        assert_eq!(resolver.resolve_provider("qwen2.5-coder-32b"), "dashscope");
    }

    #[test]
    fn resolver_routes_deepseek_models_to_deepseek() {
        let resolver = ResolverChain::built_in();
        assert_eq!(resolver.resolve_provider("deepseek-chat"), "deepseek");
        assert_eq!(resolver.resolve_provider("deepseek-coder"), "deepseek");
    }

    #[test]
    fn resolver_routes_slash_models_to_openrouter_first() {
        let resolver = ResolverChain::built_in();
        assert_eq!(
            resolver.resolve_provider("qwen/qwen3.6-plus:free"),
            "openrouter"
        );
        assert_eq!(resolver.resolve_provider("openai/gpt-4o"), "openrouter");
        assert_eq!(
            resolver.resolve_provider("anthropic/claude-3.5-sonnet"),
            "openrouter"
        );
        assert_eq!(
            resolver.resolve_provider("meta-llama/llama-3-70b"),
            "openrouter"
        );
    }

    #[test]
    fn resolver_routes_minimax_case_insensitively() {
        let resolver = ResolverChain::built_in();
        assert_eq!(resolver.resolve_provider("MiniMax-M2.7"), "minimax");
        assert_eq!(
            resolver.resolve_provider("MiniMax-M2.7-highspeed"),
            "minimax"
        );
        assert_eq!(resolver.resolve_provider("minimax-m2"), "minimax");
    }

    #[test]
    fn resolver_falls_back_to_model_reference() {
        let resolver = ResolverChain::built_in();
        assert_eq!(resolver.resolve_provider("llama-3"), "llama-3");
    }

    #[test]
    fn resolver_accepts_descriptor_supplied_routes() {
        let resolver = ResolverChain::from_descriptors(vec![
            ModelRouteDescriptor::prefix("local-", "local-provider"),
            ModelRouteDescriptor::case_insensitive_prefix("Edge-", "edge-provider"),
        ]);

        assert_eq!(resolver.resolve_provider("local-small"), "local-provider");
        assert_eq!(resolver.resolve_provider("edge-fast"), "edge-provider");
        assert_eq!(resolver.resolve_provider("unknown-model"), "unknown-model");
    }
}
