/// Resolves a provider id from a model reference.
pub trait ProviderResolver: Send + Sync {
    fn resolve(&self, model: &str) -> Option<String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PrefixMatcher {
    Slash,
    Prefix(&'static str),
    CaseInsensitivePrefix(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrefixRule {
    matcher: PrefixMatcher,
    provider: &'static str,
}

/// Resolves model families through ordered prefix-style rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixProviderResolver {
    rules: Vec<PrefixRule>,
}

impl PrefixProviderResolver {
    pub fn built_in() -> Self {
        Self {
            rules: vec![
                PrefixRule {
                    matcher: PrefixMatcher::Slash,
                    provider: "openrouter",
                },
                PrefixRule {
                    matcher: PrefixMatcher::Prefix("gpt-"),
                    provider: "openai",
                },
                PrefixRule {
                    matcher: PrefixMatcher::Prefix("o1"),
                    provider: "openai",
                },
                PrefixRule {
                    matcher: PrefixMatcher::Prefix("o3"),
                    provider: "openai",
                },
                PrefixRule {
                    matcher: PrefixMatcher::Prefix("claude-"),
                    provider: "anthropic",
                },
                PrefixRule {
                    matcher: PrefixMatcher::Prefix("qwen"),
                    provider: "dashscope",
                },
                PrefixRule {
                    matcher: PrefixMatcher::Prefix("deepseek-"),
                    provider: "deepseek",
                },
                PrefixRule {
                    matcher: PrefixMatcher::CaseInsensitivePrefix("minimax-"),
                    provider: "minimax",
                },
            ],
        }
    }
}

impl ProviderResolver for PrefixProviderResolver {
    fn resolve(&self, model: &str) -> Option<String> {
        self.rules
            .iter()
            .find(|rule| match rule.matcher {
                PrefixMatcher::Slash => model.contains('/'),
                PrefixMatcher::Prefix(prefix) => model.starts_with(prefix),
                PrefixMatcher::CaseInsensitivePrefix(prefix) => model
                    .get(..prefix.len())
                    .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix)),
            })
            .map(|rule| rule.provider.to_string())
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
}
