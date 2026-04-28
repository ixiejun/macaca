# Macaca Driver 开发指南

## 概述

### Driver 是什么

在 Macaca Agent OS 中，**Driver（驱动）** 是连接 Agent 与外部软件的桥梁。每个 Driver 将一个外部软件（CLI 工具、REST API、GUI 应用等）的能力封装为一组标准化的 **Tool（工具）**，使 Agent 能够通过统一的接口操控任意软件。

类比操作系统：Driver 之于 Agent OS，正如设备驱动之于 Linux —— Agent 不需要知道底层软件的细节，只需调用 Driver 暴露的 Tool 即可。

### 插件化架构

Macaca 采用**动态链接库插件架构**，Driver 编译为独立的共享库文件：

- macOS: `.dylib`
- Linux: `.so`

Driver 插件安装到 Macaca 工作目录的 `drivers/` 子目录中，OS 在运行时通过 C-ABI 接口动态加载。这意味着：

- **独立编译**：Driver 可以独立于 Macaca 主程序编译和发布
- **热加载**：通过 API 调用即可加载新 Driver，无需重启 OS
- **版本隔离**：每个 Driver 有独立的版本号和 ABI 版本检查
- **安全边界**：通过 C-ABI 接口通信，避免 Rust ABI 不稳定性问题

### Driver 的生命周期

```
加载（load）→ 创建实例（create）→ 获取 Manifest → 暴露工具（tools）→ 执行工具（execute）→ 健康检查（health_check）→ 关闭（shutdown）→ 销毁（destroy）
```

1. **加载**：OS 扫描 `drivers/` 目录，读取 `driver.toml`，加载 `.dylib/.so`
2. **创建实例**：调用 `macaca_driver_create(config_json)` 创建 Driver 实例
3. **获取 Manifest**：调用 `macaca_driver_manifest()` 获取 Driver 元数据
4. **暴露工具**：调用 `macaca_driver_tool_definitions()` 获取工具定义列表
5. **执行工具**：Agent 调用工具时，通过 `macaca_driver_execute_tool()` 执行
6. **健康检查**：定期调用 `macaca_driver_health_check()` 检查 Driver 状态
7. **关闭**：调用 `macaca_driver_shutdown()` 优雅关闭
8. **销毁**：调用 `macaca_driver_destroy()` 释放资源

---

## 快速开始

以下演示从零创建一个最简单的 "Hello World" Driver。

### 1. 创建项目

```bash
cargo new --lib my-hello-driver
cd my-hello-driver
```

### 2. 配置 Cargo.toml

```toml
[package]
name = "my-hello-driver"
version = "0.1.0"
edition = "2021"

[lib]
# 必须同时包含 cdylib（生成 .dylib/.so）和 rlib（支持单元测试）
crate-type = ["cdylib", "rlib"]

[dependencies]
macaca-driver = { path = "../macaca/crates/macaca-driver" }
macaca-tools = { path = "../macaca/crates/macaca-tools" }
macaca-proto = { path = "../macaca/crates/macaca-proto" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
```

> **注意**：如果你的 Driver 项目在 Macaca workspace 外部，请将 `path` 替换为 git 依赖或发布的 crate 版本。

### 3. 实现 Driver

编辑 `src/lib.rs`：

```rust
//! Hello World Driver — 最简 Macaca Driver 示例

pub mod driver;
pub mod tools;

pub use driver::HelloDriver;

use serde::Deserialize;

/// Driver 配置
#[derive(Debug, Clone, Deserialize)]
pub struct HelloConfig {
    /// 问候语前缀，默认为 "你好"
    #[serde(default = "default_greeting")]
    pub greeting: String,
}

fn default_greeting() -> String {
    "你好".into()
}

// 导出 C-ABI 入口点 —— 这是插件化的关键！
macaca_driver::export_driver!(HelloDriver, |config: serde_json::Value| {
    let config: HelloConfig = serde_json::from_value(config)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(Box::new(HelloDriver::new(config)))
});
```

编辑 `src/driver.rs`：

```rust
//! HelloDriver 实现

use async_trait::async_trait;

use macaca_driver::driver::{DriverManifest, DriverType, SoftwareDriver};
use macaca_proto::{DriverId, MacacaResult};
use macaca_tools::Tool;

use crate::HelloConfig;
use crate::tools::HelloTool;

pub struct HelloDriver {
    manifest: DriverManifest,
    config: HelloConfig,
}

impl HelloDriver {
    pub fn new(config: HelloConfig) -> Self {
        Self {
            manifest: DriverManifest {
                id: DriverId::new(),
                name: "hello-world".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                driver_type: DriverType::RestApi,
                description: "一个最简单的示例 Driver".into(),
                capabilities: vec!["greet".into()],
            },
            config,
        }
    }
}

#[async_trait]
impl SoftwareDriver for HelloDriver {
    fn manifest(&self) -> &DriverManifest {
        &self.manifest
    }

    async fn initialize(&mut self) -> MacacaResult<()> {
        tracing::info!(driver = "hello-world", "Hello Driver 已初始化");
        Ok(())
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![Box::new(HelloTool {
            greeting: self.config.greeting.clone(),
        })]
    }

    async fn health_check(&self) -> MacacaResult<bool> {
        Ok(true) // 始终健康
    }

    async fn shutdown(&mut self) -> MacacaResult<()> {
        tracing::info!(driver = "hello-world", "Hello Driver 已关闭");
        Ok(())
    }
}
```

编辑 `src/tools.rs`：

```rust
//! Hello Tool 实现

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use macaca_tools::Tool;
use serde_json::{json, Value};

pub struct HelloTool {
    pub greeting: String,
}

#[async_trait]
impl Tool for HelloTool {
    fn name(&self) -> &str {
        "hello_greet"
    }

    fn description(&self) -> &str {
        "向指定的人打招呼"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "要问候的人的名字"
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let name = input["name"].as_str().unwrap_or("世界");
        Ok(json!({
            "message": format!("{}，{}！", self.greeting, name)
        }))
    }
}
```

### 4. 创建 driver.toml

在项目根目录创建 `driver.toml`：

```toml
[driver]
name = "hello-world"
version = "0.1.0"
description = "一个最简单的示例 Driver"
library = "libmy_hello_driver.dylib"   # Linux 上改为 libmy_hello_driver.so
min_abi_version = 1

[config]
greeting = "你好"
```

### 5. 编译

```bash
cargo build --release
```

### 6. 安装

```bash
# 在 Macaca 工作目录下创建 Driver 目录
mkdir -p /path/to/macaca/drivers/hello-world

# 复制文件
cp target/release/libmy_hello_driver.dylib /path/to/macaca/drivers/hello-world/
cp driver.toml /path/to/macaca/drivers/hello-world/
```

### 7. 加载

调用 API 重新加载：

```bash
curl -X POST http://localhost:3001/api/drivers/reload
```

验证加载成功：

```bash
curl http://localhost:3001/api/drivers
```

---

## 核心概念

### SoftwareDriver Trait

`SoftwareDriver` 是每个 Driver 必须实现的核心 trait，定义在 `macaca-driver` crate 中。

```rust
#[async_trait]
pub trait SoftwareDriver: Send + Sync {
    /// 返回 Driver 的元数据（名称、版本、类型、能力列表等）
    fn manifest(&self) -> &DriverManifest;

    /// 初始化 Driver（启动子进程、连接 API 等）
    async fn initialize(&mut self) -> MacacaResult<()>;

    /// 暴露 Driver 提供的工具列表
    fn tools(&self) -> Vec<Box<dyn Tool>>;

    /// 健康检查 —— 检查 Driver 及其目标软件是否正常
    async fn health_check(&self) -> MacacaResult<bool>;

    /// 优雅关闭 —— 关闭连接、终止子进程等
    async fn shutdown(&mut self) -> MacacaResult<()>;
}
```

#### 方法说明

| 方法 | 调用时机 | 说明 |
|------|----------|------|
| `manifest()` | 加载后立即调用 | 返回 `&DriverManifest` 引用，包含 Driver 的静态元数据 |
| `initialize()` | 首次使用前 | 执行一次性初始化操作（启动进程、建立连接等） |
| `tools()` | 每次 Agent 构建工具集时 | 返回当前可用的 Tool 列表 |
| `health_check()` | 周期性调用 | 返回 `Ok(true)` 表示健康，`Ok(false)` 表示不健康 |
| `shutdown()` | OS 关闭或 Driver 卸载时 | 释放所有资源，确保无泄漏 |

### DriverManifest 结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverManifest {
    pub id: DriverId,           // 唯一标识符（UUID，自动生成）
    pub name: String,           // Driver 名称（如 "claude-code"）
    pub version: String,        // 版本号（如 "0.1.0"）
    pub driver_type: DriverType, // Driver 类型
    pub description: String,    // 人类可读的描述
    pub capabilities: Vec<String>, // 能力标签列表
}
```

### DriverType 枚举

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriverType {
    CliSubprocess,   // 通过子进程控制 CLI 程序（stdin/stdout）
    RestApi,         // 通过 REST 或 GraphQL API 交互
    UiAutomation,    // 通过辅助功能/AppleScript 控制 GUI 应用
    FileIpc,         // 通过文件或 IPC 管道通信
    McpProtocol,     // 连接 MCP 服务器
}
```

选择与你的 Driver 交互方式最匹配的类型。

### Tool Trait

每个 Driver 通过 `tools()` 方法暴露一组 `Tool`。`Tool` trait 定义在 `macaca-tools` crate 中。

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具名称（唯一标识符，Agent 通过此名称调用工具）
    fn name(&self) -> &str;

    /// 工具描述（LLM 用此描述理解工具用途）
    fn description(&self) -> &str;

    /// 输入参数的 JSON Schema（LLM 用此生成正确的参数）
    fn parameters_schema(&self) -> Value;

    /// 执行工具，接收 JSON 输入，返回 JSON 输出
    async fn execute(&self, input: Value) -> MacacaResult<Value>;

    /// 带流式事件的执行（可选，默认委托给 execute）
    async fn execute_streaming(
        &self,
        input: Value,
        event_tx: Option<UnboundedSender<TraceEvent>>,
    ) -> MacacaResult<Value> {
        let _ = event_tx;
        self.execute(input).await
    }
}
```

#### parameters_schema 格式

`parameters_schema()` 必须返回一个符合 [JSON Schema](https://json-schema.org/) 规范的 `serde_json::Value`。示例：

```rust
fn parameters_schema(&self) -> Value {
    json!({
        "type": "object",
        "properties": {
            "url": {
                "type": "string",
                "description": "要请求的 URL 地址"
            },
            "method": {
                "type": "string",
                "enum": ["GET", "POST", "PUT", "DELETE"],
                "description": "HTTP 方法",
                "default": "GET"
            },
            "body": {
                "type": "string",
                "description": "请求体（仅 POST/PUT 需要）"
            }
        },
        "required": ["url"]
    })
}
```

#### execute 方法

- **输入**：`serde_json::Value`，结构符合 `parameters_schema` 定义
- **输出**：`MacacaResult<Value>`，成功时返回任意 JSON 值，失败时返回 `MacacaError`

---

## SDK 使用

### export_driver! 宏

`export_driver!` 宏是开发 Driver 插件的核心工具，它自动生成所有 C-ABI 导出函数，开发者只需专注于 `SoftwareDriver` 实现。

#### 签名

```rust
macaca_driver::export_driver!($driver_type:ty, $create_fn:expr);
```

#### 参数说明

| 参数 | 类型 | 说明 |
|------|------|------|
| `$driver_type` | 类型名 | 你的 Driver 具体类型（用于文档和错误信息） |
| `$create_fn` | 闭包 | `fn(serde_json::Value) -> Result<Box<dyn SoftwareDriver>, Box<dyn std::error::Error>>`，从 JSON 配置构造 Driver 实例 |

#### 使用示例

```rust
macaca_driver::export_driver!(MyDriver, |config: serde_json::Value| {
    let cfg: MyConfig = serde_json::from_value(config)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(Box::new(MyDriver::new(cfg)))
});
```

#### 宏展开后生成的内容

宏展开后会生成以下 9 个 `#[no_mangle] extern "C"` 函数：

1. `macaca_driver_abi_version()` — 返回 ABI 版本号
2. `macaca_driver_create(config_json)` — 创建 Driver 实例
3. `macaca_driver_manifest(handle)` — 返回 Manifest JSON
4. `macaca_driver_tool_definitions(handle)` — 返回工具定义 JSON 数组
5. `macaca_driver_execute_tool(handle, tool_name, input_json)` — 执行工具
6. `macaca_driver_health_check(handle)` — 健康检查
7. `macaca_driver_shutdown(handle)` — 优雅关闭
8. `macaca_driver_destroy(handle)` — 销毁实例并释放内存
9. `macaca_driver_free_string(s)` — 释放由 Driver 分配的字符串

宏内部使用一个全局 `Mutex<Option<Box<dyn SoftwareDriver>>>` 存储 Driver 实例，所有 extern "C" 函数通过该全局变量访问 Driver。异步方法通过 `tokio::runtime::Builder::new_current_thread()` 桥接到同步 FFI 调用。

### Cargo.toml 配置

#### 必须的 crate-type

```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

- `cdylib`：生成动态链接库（`.dylib`/`.so`），这是插件加载必需的
- `rlib`：保留 Rust 静态库格式，使单元测试和 `cargo test` 正常工作

#### 必须的依赖

```toml
[dependencies]
macaca-driver = { version = "0.1.0" }  # Driver trait 和 SDK 宏
macaca-tools = { version = "0.1.0" }   # Tool trait
macaca-proto = { version = "0.1.0" }   # 共享类型（MacacaResult, DriverId 等）
serde = { version = "1", features = ["derive"] }  # 序列化
serde_json = "1"                        # JSON 处理
async-trait = "0.1"                     # 异步 trait 支持
tokio = { version = "1", features = ["full"] }  # 异步运行时
```

#### 推荐的依赖

```toml
[dependencies]
tracing = "0.1"  # 结构化日志（推荐，与 OS 日志系统集成）
```

---

## C-ABI 接口参考

### ABI 版本

当前 ABI 版本：**1**（定义为 `DRIVER_ABI_VERSION` 常量）

ABI 版本在发生破坏性变更时递增。宿主端会检查 Driver 报告的 ABI 版本：如果 Driver 的 ABI 版本低于宿主要求的最低版本，将拒绝加载。

### 9 个导出函数详解

#### 1. macaca_driver_abi_version

```c
uint32_t macaca_driver_abi_version(void);
```

- **用途**：返回 Driver 实现的 ABI 版本号
- **返回值**：ABI 版本号（当前为 1）
- **内存**：无分配

#### 2. macaca_driver_create

```c
void* macaca_driver_create(const char* config_json);
```

- **用途**：根据 JSON 配置创建 Driver 实例
- **参数**：`config_json` — 以 null 结尾的 JSON 字符串，内容来自 `driver.toml` 的 `[config]` 段
- **返回值**：成功返回非空 opaque handle，失败返回 `NULL`
- **内存**：handle 由 Driver 内部管理（全局 Mutex），无需调用方释放

#### 3. macaca_driver_manifest

```c
char* macaca_driver_manifest(void* handle);
```

- **用途**：获取 Driver 的 Manifest 信息
- **参数**：`handle` — `create` 返回的 handle
- **返回值**：JSON 字符串（`DriverManifestAbi` 格式），失败返回 `NULL`
- **内存**：**调用方必须通过 `macaca_driver_free_string` 释放返回的字符串**

#### 4. macaca_driver_tool_definitions

```c
char* macaca_driver_tool_definitions(void* handle);
```

- **用途**：获取 Driver 暴露的工具定义列表
- **参数**：`handle` — `create` 返回的 handle
- **返回值**：JSON 数组字符串（`Vec<ToolDefinitionAbi>` 格式），失败返回 `NULL`
- **内存**：**调用方必须通过 `macaca_driver_free_string` 释放返回的字符串**

#### 5. macaca_driver_execute_tool

```c
char* macaca_driver_execute_tool(void* handle, const char* tool_name, const char* input_json);
```

- **用途**：执行指定工具
- **参数**：
  - `handle` — `create` 返回的 handle
  - `tool_name` — 工具名称（以 null 结尾的 C 字符串）
  - `input_json` — 工具输入参数（以 null 结尾的 JSON 字符串）
- **返回值**：JSON 字符串（`ToolResultAbi` 格式），失败返回 `NULL`
- **内存**：**调用方必须通过 `macaca_driver_free_string` 释放返回的字符串**

#### 6. macaca_driver_health_check

```c
int macaca_driver_health_check(void* handle);
```

- **用途**：检查 Driver 健康状态
- **参数**：`handle` — `create` 返回的 handle
- **返回值**：`1` = 健康，`0` = 不健康，`-1` = 错误
- **内存**：无分配

#### 7. macaca_driver_shutdown

```c
void macaca_driver_shutdown(void* handle);
```

- **用途**：优雅关闭 Driver（关闭连接、终止子进程等）
- **参数**：`handle` — `create` 返回的 handle
- **内存**：不释放 handle，只做逻辑关闭

#### 8. macaca_driver_destroy

```c
void macaca_driver_destroy(void* handle);
```

- **用途**：销毁 Driver 实例，释放所有内存
- **参数**：`handle` — `create` 返回的 handle
- **内存**：释放 Driver 实例（将全局 Mutex 中的 Option 设为 None）

#### 9. macaca_driver_free_string

```c
void macaca_driver_free_string(char* s);
```

- **用途**：释放由 Driver 分配的字符串
- **参数**：`s` — 由 `manifest`、`tool_definitions`、`execute_tool` 返回的字符串指针
- **内存**：通过 `CString::from_raw(s)` 回收内存

### JSON 交换格式

#### ToolDefinitionAbi

```json
{
    "name": "工具名称",
    "description": "工具描述",
    "parameters_schema": {
        "type": "object",
        "properties": { ... },
        "required": [ ... ]
    }
}
```

#### ToolResultAbi

```json
{
    "success": true,
    "output": { "任意": "JSON 值" },
    "error": null
}
```

失败时：

```json
{
    "success": false,
    "output": null,
    "error": "错误描述信息"
}
```

#### DriverManifestAbi

```json
{
    "name": "driver-name",
    "version": "0.1.0",
    "driver_type": "CliSubprocess",
    "description": "Driver 描述",
    "capabilities": ["capability1", "capability2"]
}
```

`driver_type` 的合法字符串值：`"CliSubprocess"`, `"RestApi"`, `"UiAutomation"`, `"FileIpc"`, `"McpProtocol"`。

### 内存管理规则

**核心原则：谁分配谁释放。**

- Driver 返回的字符串（`*mut c_char`）由 Driver 内部通过 `CString::into_raw()` 分配
- 宿主端读取完字符串内容后，**必须**调用 `macaca_driver_free_string()` 让 Driver 释放该内存
- `handle` 是一个 sentinel 值（非堆分配指针），其背后的实际 Driver 实例存储在全局 Mutex 中，通过 `macaca_driver_destroy()` 释放

---

## driver.toml Manifest

每个 Driver 插件必须包含一个 `driver.toml` 文件，描述 Driver 的元数据和默认配置。

### 完整格式

```toml
[driver]
name = "my-driver"              # 必填：Driver 名称，建议使用 kebab-case
version = "0.1.0"               # 必填：版本号（语义化版本）
description = "Driver 的简短描述"  # 必填：人类可读的描述
library = "libmy_driver.dylib"  # 必填：共享库文件名
min_abi_version = 1             # 可选：要求的最低 ABI 版本，默认为 1

[config]
# 可选段：任意 key-value 配置
# 这些配置会被序列化为 JSON 传递给 macaca_driver_create()
key1 = "value1"
key2 = 42
nested_key = { sub_key = "sub_value" }
```

### 字段说明

| 段 | 字段 | 类型 | 必填 | 默认值 | 说明 |
|-----|------|------|------|--------|------|
| `[driver]` | `name` | String | 是 | - | Driver 唯一名称 |
| `[driver]` | `version` | String | 是 | - | 语义化版本号 |
| `[driver]` | `description` | String | 是 | - | 简短描述 |
| `[driver]` | `library` | String | 是 | - | 共享库文件名（相对于 Driver 目录） |
| `[driver]` | `min_abi_version` | u32 | 否 | 1 | 要求宿主支持的最低 ABI 版本 |
| `[config]` | * | Any | 否 | `{}` | 传递给 Driver 的自定义配置 |

### [config] 段的用途

`[config]` 段的内容会被整体序列化为 JSON 字符串，传递给 `macaca_driver_create(config_json)` 函数。你的 Driver 可以将其反序列化为自定义的配置结构体。

参考 claude-code driver 的示例：

```toml
[config]
claude_bin = "claude"
work_dir = "/tmp/macaca-workspace"
timeout_secs = 600
```

对应的配置结构体：

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeCodeConfig {
    pub claude_bin: String,
    pub work_dir: PathBuf,
    pub timeout_secs: u64,
    // ... 更多字段
}
```

---

## 编译与安装

### 编译

#### Release 构建

```bash
cargo build --release
```

产出文件位置：

- macOS: `target/release/lib<crate_name>.dylib`
- Linux: `target/release/lib<crate_name>.so`

> **注意**：crate 名称中的连字符 `-` 在库文件名中会替换为下划线 `_`。例如 crate 名 `my-hello-driver` 生成 `libmy_hello_driver.dylib`。

#### 跨平台编译

如果你需要在 macOS 上编译 Linux 版本（或反之），可以使用交叉编译：

```bash
# 添加目标平台
rustup target add x86_64-unknown-linux-gnu

# 交叉编译（需要配置 linker）
cargo build --release --target x86_64-unknown-linux-gnu
```

推荐使用 [cross](https://github.com/cross-rs/cross) 工具简化交叉编译：

```bash
cargo install cross
cross build --release --target x86_64-unknown-linux-gnu
```

### 安装

#### drivers 目录结构

```
drivers/
├── hello-world/              # 每个 Driver 一个子目录
│   ├── driver.toml           # Manifest 文件（必须）
│   └── libmy_hello_driver.dylib  # 共享库文件（必须）
├── claude-code/
│   ├── driver.toml
│   └── libmacaca_driver_claude_code.dylib
└── another-driver/
    ├── driver.toml
    └── libanother_driver.so
```

`drivers/` 目录的位置通过以下方式确定（优先级从高到低）：

1. 环境变量 `MACACA_DRIVERS_DIR`
2. 配置文件 `config/default.toml` 中的 `drivers.directory`

#### 安装步骤

```bash
# 1. 创建 Driver 子目录
mkdir -p /path/to/macaca/drivers/my-driver

# 2. 复制共享库
cp target/release/libmy_driver.dylib /path/to/macaca/drivers/my-driver/

# 3. 复制 manifest
cp driver.toml /path/to/macaca/drivers/my-driver/

# 4. 触发重新加载
curl -X POST http://localhost:3001/api/drivers/reload
```

#### 验证安装

```bash
# 查看已加载的 Driver 列表
curl http://localhost:3001/api/drivers | jq
```

成功响应示例：

```json
{
  "drivers": [
    {
      "name": "my-driver",
      "version": "0.1.0",
      "driver_type": "RestApi",
      "description": "我的自定义 Driver",
      "capabilities": ["my_capability"],
      "tools_count": 0
    }
  ],
  "total": 1
}
```

### 卸载

卸载 Driver 非常简单：

1. 删除 `drivers/` 下对应的子目录
2. 调用重新加载 API 使变更生效

```bash
rm -rf /path/to/macaca/drivers/my-driver
curl -X POST http://localhost:3001/api/drivers/reload
```

---

## API 管理

### GET /api/drivers — 查看已加载 Driver

```bash
curl http://localhost:3001/api/drivers
```

**响应格式**：

```json
{
  "drivers": [
    {
      "name": "string",
      "version": "string",
      "driver_type": "string",
      "description": "string",
      "capabilities": ["string"],
      "tools_count": 0
    }
  ],
  "total": 1
}
```

### POST /api/drivers/reload — 重新加载

重新扫描 `drivers/` 目录，加载所有发现的 Driver 插件。

```bash
curl -X POST http://localhost:3001/api/drivers/reload
```

**响应格式**：

```json
{
  "loaded": 2,
  "failed": 0,
  "results": [
    {
      "name": "hello-world",
      "status": "ok"
    },
    {
      "name": "claude-code",
      "status": "ok"
    }
  ]
}
```

如果某个 Driver 加载失败：

```json
{
  "loaded": 1,
  "failed": 1,
  "results": [
    {
      "name": "good-driver",
      "status": "ok"
    },
    {
      "name": "bad-driver",
      "status": "error",
      "error": "Missing symbol macaca_driver_abi_version: ..."
    }
  ]
}
```

> **注意**：reload 后，新加载的 Driver 工具会在 Agent 下一次构建工具集时自动可用，无需重启 OS。

---

## 完整示例：开发一个 HTTP API Driver

以下示例展示如何开发一个调用外部 HTTP API 的 Driver，提供天气查询功能。

### 项目结构

```
weather-driver/
├── Cargo.toml
├── driver.toml
└── src/
    ├── lib.rs
    ├── config.rs
    ├── driver.rs
    └── tools.rs
```

### Cargo.toml

```toml
[package]
name = "weather-driver"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
macaca-driver = { version = "0.1.0" }
macaca-tools = { version = "0.1.0" }
macaca-proto = { version = "0.1.0" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
reqwest = { version = "0.11", features = ["json"] }
```

### driver.toml

```toml
[driver]
name = "weather"
version = "0.1.0"
description = "天气查询 Driver —— 通过 HTTP API 获取天气信息"
library = "libweather_driver.dylib"
min_abi_version = 1

[config]
api_base_url = "https://api.weatherapi.com/v1"
api_key = "YOUR_API_KEY_HERE"
timeout_secs = 30
```

### src/config.rs

```rust
//! 天气 Driver 配置

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct WeatherConfig {
    /// API 基础 URL
    pub api_base_url: String,

    /// API 密钥
    pub api_key: String,

    /// 请求超时（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    30
}
```

### src/driver.rs

```rust
//! WeatherDriver 实现

use async_trait::async_trait;

use macaca_driver::driver::{DriverManifest, DriverType, SoftwareDriver};
use macaca_proto::{DriverId, MacacaResult};
use macaca_tools::Tool;

use crate::config::WeatherConfig;
use crate::tools::WeatherQueryTool;

pub struct WeatherDriver {
    manifest: DriverManifest,
    config: WeatherConfig,
}

impl WeatherDriver {
    pub fn new(config: WeatherConfig) -> Self {
        Self {
            manifest: DriverManifest {
                id: DriverId::new(),
                name: "weather".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                driver_type: DriverType::RestApi,
                description: "通过 HTTP API 查询天气信息".into(),
                capabilities: vec!["query_weather".into()],
            },
            config,
        }
    }
}

#[async_trait]
impl SoftwareDriver for WeatherDriver {
    fn manifest(&self) -> &DriverManifest {
        &self.manifest
    }

    async fn initialize(&mut self) -> MacacaResult<()> {
        tracing::info!(
            driver = "weather",
            api_base = %self.config.api_base_url,
            "Weather Driver 已初始化"
        );
        Ok(())
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![Box::new(WeatherQueryTool {
            api_base_url: self.config.api_base_url.clone(),
            api_key: self.config.api_key.clone(),
            timeout_secs: self.config.timeout_secs,
        })]
    }

    async fn health_check(&self) -> MacacaResult<bool> {
        // 简单检查：尝试请求 API 根路径
        let client = reqwest::Client::new();
        let resp = client
            .get(&self.config.api_base_url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;
        Ok(resp.is_ok())
    }

    async fn shutdown(&mut self) -> MacacaResult<()> {
        tracing::info!(driver = "weather", "Weather Driver 已关闭");
        Ok(())
    }
}
```

### src/tools.rs

```rust
//! 天气查询工具

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use macaca_tools::Tool;
use serde_json::{json, Value};

pub struct WeatherQueryTool {
    pub api_base_url: String,
    pub api_key: String,
    pub timeout_secs: u64,
}

#[async_trait]
impl Tool for WeatherQueryTool {
    fn name(&self) -> &str {
        "weather_query"
    }

    fn description(&self) -> &str {
        "查询指定城市的当前天气信息，包括温度、湿度、天气状况等"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "城市名称（支持中文和英文，如 '北京' 或 'Beijing'）"
                },
                "lang": {
                    "type": "string",
                    "description": "返回结果的语言，默认 'zh'",
                    "default": "zh"
                }
            },
            "required": ["city"]
        })
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let city = input["city"]
            .as_str()
            .ok_or_else(|| macaca_proto::MacacaError::Tool("缺少 city 参数".into()))?;

        let lang = input["lang"].as_str().unwrap_or("zh");

        let client = reqwest::Client::new();
        let url = format!("{}/current.json", self.api_base_url);

        let resp = client
            .get(&url)
            .query(&[("key", &self.api_key), ("q", &city.to_string()), ("lang", &lang.to_string())])
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .send()
            .await
            .map_err(|e| macaca_proto::MacacaError::Tool(format!("HTTP 请求失败: {}", e)))?;

        if !resp.status().is_success() {
            return Err(macaca_proto::MacacaError::Tool(format!(
                "API 返回错误状态: {}",
                resp.status()
            )));
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| macaca_proto::MacacaError::Tool(format!("JSON 解析失败: {}", e)))?;

        Ok(body)
    }
}
```

### src/lib.rs

```rust
//! Weather Driver — 天气查询 Macaca Driver

pub mod config;
pub mod driver;
pub mod tools;

pub use config::WeatherConfig;
pub use driver::WeatherDriver;

// 导出 C-ABI 入口点
macaca_driver::export_driver!(WeatherDriver, |config: serde_json::Value| {
    let config: WeatherConfig = serde_json::from_value(config)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(Box::new(WeatherDriver::new(config)))
});
```

### 编译与安装

```bash
# 编译
cargo build --release

# 安装
mkdir -p /path/to/macaca/drivers/weather
cp target/release/libweather_driver.dylib /path/to/macaca/drivers/weather/
cp driver.toml /path/to/macaca/drivers/weather/

# 加载
curl -X POST http://localhost:3001/api/drivers/reload
```

---

## 最佳实践

### 错误处理

1. **永远不要 panic**：所有 extern "C" 函数已被 `catch_unwind` 包裹，但仍应避免 panic

2. **使用 `MacacaResult`**：统一使用 `macaca_proto::MacacaResult<T>` 作为返回类型

3. **提供有意义的错误信息**：

```rust
// 好的做法
Err(MacacaError::Tool(format!(
    "无法连接到 {} : 超时 {} 秒", url, timeout
)))

// 不好的做法
Err(MacacaError::Tool("error".into()))
```

4. **在 `create_fn` 闭包中处理配置错误**：

```rust
macaca_driver::export_driver!(MyDriver, |config: serde_json::Value| {
    let cfg: MyConfig = serde_json::from_value(config)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    // 验证配置
    if cfg.api_key.is_empty() {
        return Err("api_key 不能为空".into());
    }
    Ok(Box::new(MyDriver::new(cfg)))
});
```

### 日志记录

使用 `tracing` crate 记录结构化日志，与 Macaca OS 的日志系统无缝集成：

```rust
use tracing::{info, warn, error, debug};

// 带上下文的结构化日志
info!(driver = "my-driver", action = "initialize", "Driver 正在初始化");
warn!(tool = "my_tool", input = ?input, "输入参数异常");
error!(error = %e, "工具执行失败");
debug!(response = ?resp, "API 响应");
```

### 配置管理

1. **使用 `serde` 反序列化**：定义一个 `Config` 结构体，从 `driver.toml` 的 `[config]` 段读取

2. **提供合理的默认值**：使用 `#[serde(default = "...")]`

3. **验证配置**：在 `create_fn` 中尽早验证，避免运行时错误

4. **敏感信息**：API 密钥等敏感信息应放在 `driver.toml` 中，而不是硬编码

### 测试方法

#### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest() {
        let config = MyConfig { /* ... */ };
        let driver = MyDriver::new(config);
        assert_eq!(driver.manifest().name, "my-driver");
    }

    #[test]
    fn test_tools_count() {
        let config = MyConfig { /* ... */ };
        let driver = MyDriver::new(config);
        assert_eq!(driver.tools().len(), 2);
    }

    #[tokio::test]
    async fn test_tool_execute() {
        let tool = MyTool { /* ... */ };
        let input = serde_json::json!({"param": "value"});
        let result = tool.execute(input).await.unwrap();
        assert!(result["success"].as_bool().unwrap());
    }
}
```

运行测试：

```bash
cargo test
```

#### 集成测试

编写集成测试验证 Driver 完整生命周期：

```rust
#[tokio::test]
async fn test_driver_lifecycle() {
    let mut driver = MyDriver::new(test_config());

    // 初始化
    driver.initialize().await.unwrap();

    // 健康检查
    assert!(driver.health_check().await.unwrap());

    // 获取工具
    let tools = driver.tools();
    assert!(!tools.is_empty());

    // 关闭
    driver.shutdown().await.unwrap();
}
```

---

## 故障排查

### 常见问题

#### 1. "Missing symbol macaca_driver_abi_version"

**原因**：共享库中缺少 C-ABI 导出函数。

**解决**：
- 确认 `lib.rs` 中调用了 `macaca_driver::export_driver!` 宏
- 确认 `Cargo.toml` 中包含 `crate-type = ["cdylib", "rlib"]`

#### 2. "Driver ABI version X is older than required Y"

**原因**：Driver 使用的 `macaca-driver` crate 版本过旧，ABI 版本低于宿主要求。

**解决**：更新 `macaca-driver` 依赖到最新版本并重新编译。

#### 3. "Driver creation returned null handle"

**原因**：`create_fn` 闭包返回了错误。

**解决**：
- 检查 `driver.toml` 中的 `[config]` 段是否符合你的 Config 结构体定义
- 在 `create_fn` 中添加日志，排查具体的反序列化或验证错误

#### 4. "Driver library not found"

**原因**：`driver.toml` 中的 `library` 字段指定的文件不存在。

**解决**：
- 确认 `library` 字段的文件名与实际编译产出一致
- 注意 crate 名称中的 `-` 在库文件名中变为 `_`
- macOS 使用 `.dylib` 后缀，Linux 使用 `.so` 后缀

#### 5. "Failed to read drivers directory"

**原因**：`drivers/` 目录不存在或无读取权限。

**解决**：确认目录存在且有读取权限。OS 启动时会尝试自动创建该目录。

#### 6. Driver 加载成功但工具不可见

**原因**：Driver 工具在 Agent 构建工具集时动态聚合，而非静态注册。

**解决**：
- 确认 `/api/drivers` 返回了你的 Driver
- Agent 下一次执行时会自动获取新工具，无需重启
- 检查 `tools()` 方法是否返回了非空的工具列表

### 日志查看

Macaca 后端日志默认输出到 `/tmp/macaca-backend.log`：

```bash
# 实时查看日志
tail -f /tmp/macaca-backend.log

# 过滤 Driver 相关日志
tail -f /tmp/macaca-backend.log | grep -i driver
```

Driver 加载过程会输出以下关键日志：

```
INFO  Loading all drivers          dir="/path/to/drivers"
INFO  Discovered driver            name="my-driver" version="0.1.0"
INFO  Loading driver               name="my-driver" library="/path/to/lib"
INFO  Driver loaded successfully   name="my-driver" version="0.1.0"
INFO  Driver loading complete      total=1 success=1 failed=0
```

如果加载失败，会输出 `ERROR` 级别日志，包含具体的错误原因。

---

## 参考

- **claude-code Driver 参考实现**：`macaca/crates/macaca-driver-claude-code/` — 一个完整的生产级 Driver 实现
- **macaca-driver crate**：`macaca/crates/macaca-driver/` — Driver SDK、ABI 定义、加载器
- **macaca-tools crate**：`macaca/crates/macaca-tools/src/tool.rs` — Tool trait 定义
