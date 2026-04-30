# macaca-memory 设计模式渐进式重构计划

## 当前职责

`macaca-memory` 提供 memory store、session memory、vector store、embedding provider、in-memory/file-backed/vector-backed 实现。它是 Agent 长期运行和跨 session 记忆的基础层。

重点对象：

- `MemoryStore`。
- `VectorStore`。
- `EmbeddingProvider`。
- `MemoryManager`。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| memory backend | file/vector/in-memory 行为不同 | Strategy | backend 策略化 |
| embedding provider | 远程 embedding 调用需要重试、缓存、限流 | Proxy + Decorator | embedding proxy 统一横切能力 |
| memory manager | 调用侧需要理解 store/vector/embed 多个对象 | Facade | `MemoryManager` 做唯一门面 |
| embedding 重用 | 相同文本重复 embedding 浪费 | Flyweight | embedding cache/flyweight |
| session memory snapshot | 长期运行需要恢复上下文 | Memento | memory snapshot/replay |

## 小步重构计划

1. 第一切片：为 `MemoryManager` 增加 facade 方法，内部继续调用现有 store。
2. 第二切片：抽出 `EmbeddingCache`，先只做进程内缓存。
3. 第三切片：引入 `MemoryBackendFactory`，根据配置创建 backend。
4. 第四切片：增加 memory snapshot schema，用于 session resume 和 debugging。
5. 第五切片：为 vector query 增加 strategy，支持 hybrid search、metadata filter。

## 示例代码片段

```rust
pub trait MemoryBackend: Send + Sync {
    async fn put(&self, item: MemoryItem) -> Result<(), MemoryError>;
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryHit>, MemoryError>;
}

pub struct MemoryManager {
    backend: Arc<dyn MemoryBackend>,
    embedding: Arc<dyn EmbeddingProvider>,
}

impl MemoryManager {
    pub async fn remember_text(&self, scope: MemoryScope, text: String) -> Result<(), MemoryError> {
        let vector = self.embedding.embed(&text).await?;
        self.backend.put(MemoryItem { scope, text, vector }).await
    }
}
```

## 验证策略

- 后端 contract tests：in-memory、file、vector 同一套 put/search/delete 测试。
- embedding cache 引入前后比较 provider 调用次数和返回结果。
- snapshot/replay 用固定 session fixture 做黄金测试。

