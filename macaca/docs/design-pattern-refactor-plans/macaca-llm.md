# macaca-llm 设计模式渐进式重构计划

## 当前职责

`macaca-llm` 管理 LLM provider、router、OpenAI-compatible provider、resilience wrapper、cost tracking、rate limit 等。它决定 Agent OS 如何灵活接入 OpenAI、DeepSeek、Claude、兼容 API 和后续更多模型。

重点对象：

- `LlmProvider` trait。
- `LlmRouter`。
- `OpenAiCompatibleProvider`。
- `ResilientLlmWrapper`。
- `CostTracker` / `RateLimiter`。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| provider 选择 | 根据 model/provider 字符串做 prefix if/else，扩展成本高 | Chain of Responsibility + Strategy | provider resolver 链式匹配 |
| provider 创建 | 每个 provider 初始化参数不同 | Abstract Factory | `LlmProviderFactory` |
| API 协议差异 | OpenAI-compatible 与特殊 provider 混杂 | Adapter | 每个 provider 独立 request/response adapter |
| retry/rate-limit/cost | 横切能力容易嵌入 provider 内部 | Decorator / Proxy | wrapper 链组合 |
| thinking mode | DeepSeek 等 provider 有特定上下文字段约束 | Strategy | provider-specific conversation policy |

## 小步重构计划

1. 第一切片：抽出 `ProviderResolver`，把现有 prefix 匹配原样迁移进去。
2. 第二切片：将 `ResilientLlmWrapper` 拆成 retry、timeout、rate-limit、cost 四个 decorator。
3. 第三切片：为 DeepSeek thinking mode 增加 `ConversationPolicy`，处理 `reasoning_content` 回传规则。
4. 第四切片：引入 provider factory registry，应用配置只声明 provider id 和 model id。
5. 第五切片：建立 provider contract tests，覆盖 chat、tool call、stream、thinking。

## 示例代码片段

### Chain of Responsibility provider resolver

```rust
pub trait ProviderResolver: Send + Sync {
    fn resolve(&self, req: &LlmRequest) -> Option<ProviderId>;
}

pub struct PrefixProviderResolver {
    rules: Vec<(String, ProviderId)>,
}

pub struct ResolverChain {
    resolvers: Vec<Box<dyn ProviderResolver>>,
}

impl ResolverChain {
    pub fn resolve(&self, req: &LlmRequest) -> Result<ProviderId, LlmError> {
        self.resolvers
            .iter()
            .find_map(|resolver| resolver.resolve(req))
            .ok_or_else(|| LlmError::NoProvider(req.model.clone()))
    }
}
```

### Decorator wrapper 链

```rust
let provider: Arc<dyn LlmProvider> = Arc::new(OpenAiCompatibleProvider::new(config));
let provider = Arc::new(RateLimitedProvider::new(provider, limiter));
let provider = Arc::new(CostTrackingProvider::new(provider, cost_tracker));
let provider = Arc::new(RetryingProvider::new(provider, retry_policy));
```

## 验证策略

- 每个 provider 有同一套 contract tests。
- DeepSeek 报错 `reasoning_content must be passed back` 固化为回归测试。
- provider resolver 用 table-driven tests 覆盖模型名、显式 provider、默认 provider。

