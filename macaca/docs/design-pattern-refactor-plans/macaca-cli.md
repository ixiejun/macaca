# macaca-cli 设计模式渐进式重构计划

## 当前职责

`macaca-cli` 是本地命令入口，负责解析命令、初始化日志、启动 web/server、执行辅助命令。它是开发者和本地部署使用 Agent OS 的主要入口。

重点对象：

- CLI `main` / command dispatch。
- `commands` 模块。
- `logging` 初始化。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| command 分发 | 子命令逻辑容易堆在 match 中 | Command | 每个 CLI 命令实现独立 command handler |
| 启动配置 | web、analyze、runtime 参数初始化分散 | Builder | 构建 `CliRuntimeContext` |
| 日志初始化 | 不同命令可能需要不同日志策略 | Strategy | `LoggingStrategy` 根据命令选择输出格式 |
| CLI 到 web/kernel | CLI 直接知道过多下游启动细节 | Facade | 调用 `MacacaBootstrapFacade` |

## 小步重构计划

1. 第一切片：定义 `CliCommandHandler` trait，先只包一层旧 match 分支。
2. 第二切片：抽出 `CliRuntimeContext`，集中 cwd、config path、log level、env。
3. 第三切片：把 logging 初始化改为 `LoggingStrategy`，默认行为保持不变。
4. 第四切片：web 启动命令只调用 facade，不直接拼装底层状态。

## 示例代码片段

### Command handler

```rust
#[async_trait]
pub trait CliCommandHandler {
    async fn run(&self, ctx: CliRuntimeContext) -> anyhow::Result<()>;
}

pub struct WebCommandHandler {
    bootstrap: MacacaBootstrapFacade,
}

#[async_trait]
impl CliCommandHandler for WebCommandHandler {
    async fn run(&self, ctx: CliRuntimeContext) -> anyhow::Result<()> {
        self.bootstrap.start_web(ctx).await
    }
}
```

### Logging Strategy

```rust
pub trait LoggingStrategy {
    fn init(&self, level: &str) -> anyhow::Result<()>;
}

pub struct JsonLogging;
pub struct PrettyLogging;
```

## 验证策略

- 对每个子命令保留 CLI golden output。
- 用 `cargo run -- --help`、`cargo run -- web --help` 确认帮助信息不变。
- 重构时只迁移一个命令，避免一次性改动所有 CLI 入口。

