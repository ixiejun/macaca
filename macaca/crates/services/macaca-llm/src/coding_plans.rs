//! 各厂商「订阅 / Coding Plan / Token Plan」类接入时的基址修正。
//!
//! Macaca 的通用 HTTP 客户端走 **OpenAI 兼容** 路径（`/v1/chat/completions`）。
//! 部分文档会给出 Anthropic 兼容根地址，与当前实现混用会导致 404 或解析失败。

/// 将配置中的 `base_url` 规范为适合 [`crate::OpenAiCompatibleProvider`] 的形式。
///
/// - 去掉尾部 `/`
/// - **MiniMax**：若误配为 `…/anthropic`，改为官方 OpenAI 兼容根地址
///   `https://api.minimaxi.com/v1`（[文档](https://platform.minimaxi.com/docs/api-reference/text-openai-api)）；
///   [Token Plan](https://platform.minimaxi.com/docs/token-plan/intro) 与按量密钥均使用该 OpenAI 格式端点，与 Anthropic 路径不互通。
pub fn normalize_openai_compatible_base(provider_name: &str, base_url: &str) -> String {
    let u = base_url.trim().trim_end_matches('/').to_string();
    if provider_name == "minimax" && u.contains("/anthropic") {
        tracing::warn!(
            target: "macaca_llm",
            configured = %u,
            "MiniMax base_url points at Anthropic-compatible path; Macaca uses OpenAI /v1/chat/completions. \
             Use https://api.minimaxi.com/v1 for Token Plan or pay-as-you-go. Rewriting base_url."
        );
        return "https://api.minimaxi.com/v1".to_string();
    }
    u
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimax_anthropic_path_rewritten_to_openai_v1() {
        assert_eq!(
            normalize_openai_compatible_base("minimax", "https://api.minimaxi.com/anthropic",),
            "https://api.minimaxi.com/v1"
        );
    }

    #[test]
    fn other_providers_only_trim() {
        assert_eq!(
            normalize_openai_compatible_base("deepseek", "https://api.deepseek.com/v1/"),
            "https://api.deepseek.com/v1"
        );
    }
}
