# macaca-runtime-host 设计模式渐进式重构计划

## 当前职责

`macaca-runtime-host` 提供 MCP runtime、环境桥接、兼容层和 host 侧资源管理。它是“安装 MCP 服务后所有 application 都可以调用 MCP”的关键承载层。

重点对象：

- MCP runtime。
- env bridge。
- compat layer。
- runtime status/resource 管理。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| MCP server 生命周期 | 启动、连接、隔离、释放资源复杂 | Abstract Factory + State | `McpServerFactory` 和 `McpSessionState` |
| MCP 调用 | 远程协议、进程、stdio/http 错误转换复杂 | Proxy | `McpClientProxy` |
| transport | stdio/http/sse 等 transport 差异 | Bridge | transport 与 MCP capability 解耦 |
| env 注入 | application、workspace、agent env 容易混杂 | Builder | `RuntimeEnvBuilder` |
| resource cleanup | Playwright/browser 等资源可能被占用 | Command + Memento | session-scoped resource lease 可释放可恢复 |

## 小步重构计划

1. 第一切片：抽出 `McpRuntimeFacade`，web/tools 只通过 facade 调用 MCP。
2. 第二切片：定义 `McpTransport` bridge，先包装现有 stdio 逻辑。
3. 第三切片：引入 `McpSessionLease`，每个 agent task 获取独立 lease，完成后释放。
4. 第四切片：把 `--isolated`、workspace、cache dir 这类隔离参数放入 `McpServerFactory`。
5. 第五切片：增加 resource leak regression test，覆盖 Playwright browser already in use。

## 示例代码片段

```rust
pub struct McpSessionLease {
    server_id: String,
    session_id: String,
    cleanup: Vec<ResourceCleanupCommand>,
}

impl McpSessionLease {
    pub async fn release(self) -> Result<(), McpRuntimeError> {
        for cmd in self.cleanup {
            cmd.execute().await?;
        }
        Ok(())
    }
}

pub trait McpTransport: Send + Sync {
    async fn call(&self, request: McpRequest) -> Result<McpResponse, McpRuntimeError>;
}
```

## 验证策略

- 并发启动两个 Playwright MCP session，确认互不占用同一 browser profile。
- task complete/fail/timeout 都必须触发 lease release。
- application 级 MCP 可见性通过 capability gating 测试，而不是硬编码应用名。

