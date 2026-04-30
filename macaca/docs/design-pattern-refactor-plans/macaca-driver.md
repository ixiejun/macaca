# macaca-driver 设计模式渐进式重构计划

## 当前职责

`macaca-driver` 负责软件执行 driver 抽象、driver 注册、动态 driver 加载以及具体 driver tool 暴露。它是 Claude Code、OpenCode 等执行器进入 Agent OS 的扩展点。

重点对象：

- `SoftwareDriver` trait。
- `DriverRegistry`。
- `DriverLoader` / dynamic driver。
- driver tool definitions。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| driver 创建 | 内置 driver 与动态 driver 初始化路径容易分叉 | Abstract Factory | 统一 `DriverFactory` 创建入口 |
| 动态 ABI 边界 | 调用动态库时错误、版本、capability 检查分散 | Proxy | 用 `DynamicDriverProxy` 封装 ABI 和错误转换 |
| driver registry | 注册、发现、状态、capability 混合 | Facade + Registry | 对外提供稳定 registry facade |
| driver action | execute/resume/status 等动作是天然命令 | Command | 将 driver 调用封装为 `DriverCommand`，便于 trace 和 replay |
| driver trace | driver trace event 展示需要从内部实现名转换为用户可见动作 | Adapter + Visitor | 统一 driver event 到 UI trace event 的转换 |

## 小步重构计划

1. 第一切片：为内置 driver 和动态 driver 增加同名 `DriverFactory` 适配层，旧调用不变。
2. 第二切片：抽出 `DriverCommand`，先覆盖 `execute`，再覆盖 `resume/status`。
3. 第三切片：把 driver event 转换逻辑移入 `DriverTraceAdapter`，确保 UI 显示 driver 名和动作名。
4. 第四切片：动态 driver 加载增加 `DriverManifest` 校验代理，不让调用侧直接处理 ABI 细节。
5. 第五切片：为并发 driver session 增加 `DriverSessionState` 状态机，避免资源泄漏和状态误报。

## 示例代码片段

### Abstract Factory

```rust
pub trait DriverFactory: Send + Sync {
    fn driver_id(&self) -> &str;
    fn create(&self, ctx: DriverCreateContext) -> Result<Arc<dyn SoftwareDriver>, DriverError>;
}

pub struct BuiltinDriverFactory<D> {
    marker: PhantomData<D>,
}

pub struct DynamicDriverFactory {
    manifest: DriverManifest,
    loader: Arc<DriverLoader>,
}
```

### Command + trace

```rust
pub enum DriverCommand {
    Execute { driver: DriverId, input: DriverInput },
    Resume { driver: DriverId, session: DriverSessionId, input: DriverInput },
    Status { driver: DriverId, session: DriverSessionId },
}

impl DriverCommand {
    pub fn trace_label(&self, registry: &DriverRegistry) -> String {
        let driver_name = registry.display_name(self.driver_id());
        format!("{driver_name} {}", self.action_name())
    }
}
```

## 验证策略

- 用 Claude Code 与 OpenCode 各跑一次 execute/status/resume fixture。
- 对 driver trace 做 UI event snapshot，确认不会再出现只显示 `driver Bash`、`driver tool` 这类内部标签。
- 动态 driver 迁移前后对比 registry 中 driver id、tool schema、allowed tools。

