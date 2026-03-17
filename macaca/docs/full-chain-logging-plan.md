# 全链路日志系统实施计划

## 需求总结

| 维度 | 内容 |
|------|------|
| **目标** | Debug委托agent卡住 + 审计合规 + 系统监控 |
| **约束** | 生产info级别、可调debug、敏感信息脱敏 |
| **验收** | 日志完整性 + 关键字段验证 |

---

## 关键日志节点分析

基于代码库探索，当前日志覆盖情况：

| 组件 | 关键方法 | 当前日志状态 | 优先级 |
|------|----------|--------------|--------|
| **Fork管理** | `create_fork()` | ❌ 无日志 | 🔴 高 |
| **Fork管理** | `suspend_fork()` | ❌ 无日志 | 🔴 高 |
| **Fork管理** | `resume_fork()` | ✅ info级别 | 🟡 中 |
| **Hook事件** | `emit_hook_event()` | ❌ 无日志 | 🔴 高 |
| **调度** | `select_agent()` | ⚠️ debug级别 | 🟡 中 |
| **工具调用** | `execute_tool_call()` | ❌ 无日志 | 🔴 高 |
| **Agent循环** | `PausableAgenticLoop` | ⚠️ warn级别 | 🟡 中 |

---

## 技术方案

### 1. 日志字段规范

每个日志条目必须包含以下关联字段：

```rust
// 核心追踪字段
- trace_id: String      // 请求级追踪ID
- task_id: TaskId       // 任务ID
- fork_id: ForkId       // Fork ID
- agent_name: String    // Agent名称
- app_id: ApplicationId // 应用ID

// 时间戳字段
- timestamp: DateTime<Utc>
- elapsed_ms: u64       // 操作耗时

// 状态字段
- from_state: String    // 状态转换前
- to_state: String      // 状态转换后
```

### 2. 日志级别定义

| 级别 | 使用场景 | 是否脱敏 |
|------|----------|----------|
| `INFO` | 状态变化、关键操作完成 | 是 |
| `DEBUG` | 详细流程、中间状态 | 是（但可临时开启） |
| `WARN` | 可恢复的异常 | 是 |
| `ERROR` | 不可恢复的失败 | 是 |

### 3. 敏感信息脱敏规则

```rust
// API密钥/Token - 只保留前8位
"sk-abc1234567890abcdef" → "sk-abcd...****"

// Prompt内容 - 截断至200字符
"完整的用户prompt..." → "完整的用户prompt... [truncated 500 chars]"

// 文件路径 - 相对路径替代绝对路径
"/Users/quantum/Code/dev/agent/..." → "./project/..."
```

---

## 详细实施计划

### Phase 1: 基础框架（核心追踪字段）

#### 1.1 创建日志工具模块

**文件**: `macaca/crates/macaca-kernel/src/logging.rs` (新建)

```rust
//! 全链路日志追踪工具

use std::collections::HashMap;
use tracing::{info, debug, warn, error};

/// 日志上下文，用于传递追踪字段
#[derive(Debug, Clone)]
pub struct LogContext {
    pub trace_id: String,
    pub task_id: Option<String>,
    pub fork_id: Option<String>,
    pub agent_name: Option<String>,
    pub app_id: Option<String>,
}

/// 脱敏处理
pub fn mask_sensitive(text: &str) -> String {
    // API密钥脱敏
    if text.starts_with("sk-") && text.len() > 12 {
        format!("{}...****", &text[..12])
    } else {
        text.to_string()
    }
}

/// 截断长文本
pub fn truncate(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}... [truncated {} chars]", &text[..max_len], text.len() - max_len)
    } else {
        text.to_string()
    }
}

/// 记录状态转换
pub fn log_state_transition(
    ctx: &LogContext,
    component: &str,
    from: &str,
    to: &str,
    details: Option<HashMap<String, String>>,
) {
    info!(
        trace_id = %ctx.trace_id,
        task_id = ?ctx.task_id,
        fork_id = ?ctx.fork_id,
        agent_name = ?ctx.agent_name,
        app_id = ?ctx.app_id,
        component = %component,
        from_state = %from,
        to_state = %to,
        ?details,
        "State transition"
    );
}
```

#### 1.2 增强 ForkManager 日志

**文件**: `macaca/crates/macaca-kernel/src/executor/fork_manager.rs`

在以下方法中添加日志：

- `create_fork()` - Fork创建
- `suspend_fork()` - Fork挂起等待Hook
- `resume_fork()` - Fork恢复
- `emit_hook_event()` - Hook事件发送

#### 1.3 增强调度器日志

**文件**: `macaca/crates/macaca-kernel/src/scheduler.rs`

- `select_agent()` - 提升到info级别，记录决策原因

#### 1.4 增强工具调用日志

**文件**: `macaca/crates/macaca-runtime/src/agentic_loop.rs`

- `execute_tool_call()` - 记录工具名称、参数、结果

### Phase 2: 完整链路追踪

#### 2.1 在关键入口传递 TraceContext

**文件**:
- `macaca/crates/macaca-web/src/agent_runner.rs`
- `macaca/crates/macaca-kernel/src/executor/app_executor.rs`

确保 trace_id 在整个调用链中传递。

#### 2.2 Hook 事件完整日志

**文件**: `macaca/crates/macaca-web/src/hook_consumer.rs`

记录所有 Hook 事件的处理：
- ForkValidated
- DelegateCompleted
- DelegateFailed

### Phase 3: 验证与优化

#### 3.1 日志完整性检查脚本

**文件**: `scripts/verify_logs.py` (新建)

验证给定时间窗口内：
- 所有委托任务都有创建和完成日志
- 关键字段不缺失

#### 3.2 性能测试

确保日志添加后性能下降 < 5%

---

## 修改文件清单

| 文件 | 修改类型 | 修改内容 |
|------|----------|----------|
| `macaca/crates/macaca-kernel/src/logging.rs` | 新建 | 日志工具模块 |
| `macaca/crates/macaca-kernel/src/lib.rs` | 修改 | 导出logging模块 |
| `macaca/crates/macaca-kernel/src/executor/fork_manager.rs` | 修改 | 添加Fork生命周期日志 |
| `macaca/crates/macaca-kernel/src/scheduler.rs` | 修改 | 提升调度日志级别 |
| `macaca/crates/macaca-runtime/src/agentic_loop.rs` | 修改 | 添加工具调用日志 |
| `macaca/crates/macaca-web/src/hook_consumer.rs` | 修改 | 增强Hook事件日志 |
| `macaca/crates/macaca-web/src/agent_runner.rs` | 修改 | 传递TraceContext |

---

## 验收检查清单

- [ ] 每个委托任务从创建到完成都有完整日志链
- [ ] 每个日志都包含 trace_id, task_id, agent_name 等关键字段
- [ ] 敏感信息已脱敏（API密钥、长Prompt）
- [ ] 日志级别可配置（info/debug）
- [ ] 性能测试通过（下降 < 5%）

---

## 实施建议

1. **分阶段实施**：先完成Phase 1（核心日志），验证通过后再进行Phase 2
2. **配置开关**：添加 `enable_full_chain_logging` 配置，可临时关闭
3. **监控告警**：关键日志缺失时触发告警（可选）

---

## 请确认

1. 计划是否完整？是否需要调整？
2. 实施顺序是否可以接受？
3. 是否现在开始Phase 1开发？