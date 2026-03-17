# 日志系统实现计划

## 需求概述

1. 在配置文件（config/default.toml）中配置日志级别和日志文件选项
2. 运行时根据配置记录日志
3. 日志持久化到磁盘文件，永久保存
4. 方便后续 debug 和审计

## 设计方案

### 1. 配置结构扩展

**新增配置结构（macaca-proto/src/config.rs）：**

```rust
/// 日志文件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFileConfig {
    /// 是否启用文件日志
    pub enabled: bool,
    /// 日志文件目录
    pub dir: String,
    /// 日志文件名前缀
    pub prefix: String,
    /// 日志格式：json 或 text
    pub format: String,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// 日志级别：trace, debug, info, warn, error
    pub level: String,
    /// 文件日志配置
    pub file: LogFileConfig,
}
```

### 2. 配置文件示例

**config/default.toml：**

```toml
[log]
level = "info"

[log.file]
enabled = true
dir = "./logs"
prefix = "macaca"
format = "json"
```

### 3. 日志初始化逻辑

```rust
fn init_logging(config: &LogConfig) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建日志目录（如果不存在）
    // 2. 创建文件 appender（按日期滚动）
    // 3. 创建非阻塞写入器
    // 4. 配置多输出层（控制台 + 文件）
    // 5. 初始化 tracing subscriber
}
```

### 4. 日志文件命名和滚动

- 文件名格式：`{prefix}-{YYYY-MM-DD}.log`
- 示例：`macaca-2026-03-17.log`
- 滚动策略：按日期自动滚动

### 5. 日志格式

**JSON 格式（文件）：**
```json
{"timestamp":"2026-03-17T10:00:00Z","level":"INFO","target":"macaca_web","message":"Server started"}
```

**控制台格式（可读性）：**
```
2026-03-17T10:00:00Z INFO macaca_web: Server started addr=0.0.0.0:3001
```

## 需要修改的文件

| 文件 | 修改内容 |
|------|----------|
| `Cargo.toml` | 添加 `tracing-appender` 依赖 |
| `macaca-proto/src/config.rs` | 添加 `LogConfig` 和 `LogFileConfig` |
| `config/default.toml` | 添加 `[log]` 配置段 |
| `macaca-cli/src/main.rs` | 实现日志初始化逻辑 |

## 实现步骤

1. **Step 1**: 添加依赖 `tracing-appender = "0.2"`
2. **Step 2**: 扩展配置结构
3. **Step 3**: 更新配置文件
4. **Step 4**: 实现日志初始化
5. **Step 5**: 测试验证

## 待确认事项

1. 日志文件保留策略？
2. 是否需要日志压缩？
3. 是否需要按大小滚动？
