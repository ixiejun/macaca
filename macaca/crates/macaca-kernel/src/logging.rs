//! 全链路日志追踪工具
//!
//! 提供统一的日志记录接口，支持：
//! - 追踪字段传递（trace_id, task_id, fork_id等）
//! - 敏感信息脱敏
//! - 状态转换记录
//! - 性能计时

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// 日志上下文，用于传递追踪字段
#[derive(Debug, Clone)]
pub struct LogContext {
    /// 请求级追踪ID（顶层生成，贯穿整个请求链）
    pub trace_id: String,
    /// 任务ID
    pub task_id: Option<String>,
    /// Fork ID
    pub fork_id: Option<String>,
    /// Agent名称
    pub agent_name: Option<String>,
    /// 应用ID
    pub app_id: Option<String>,
    /// 会话ID
    pub session_id: Option<String>,
}

impl LogContext {
    /// 创建新的日志上下文，自动生成trace_id
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            task_id: None,
            fork_id: None,
            agent_name: None,
            app_id: Some(app_id.into()),
            session_id: None,
        }
    }

    /// 从已有trace_id创建
    pub fn with_trace_id(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            task_id: None,
            fork_id: None,
            agent_name: None,
            app_id: None,
            session_id: None,
        }
    }

    /// 设置task_id
    pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    /// 设置fork_id
    pub fn with_fork_id(mut self, fork_id: impl Into<String>) -> Self {
        self.fork_id = Some(fork_id.into());
        self
    }

    /// 设置agent_name
    pub fn with_agent_name(mut self, agent_name: impl Into<String>) -> Self {
        self.agent_name = Some(agent_name.into());
        self
    }

    /// 设置session_id
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// 敏感信息脱敏处理
pub fn mask_sensitive(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    // API密钥/Token脱敏 - 保留前8位
    if text.starts_with("sk-") && text.len() > 12 {
        return format!("{}...****", &text[..12]);
    }

    // Bearer token脱敏
    if text.starts_with("Bearer ") && text.len() > 20 {
        return format!("Bearer {}...****", &text[7..15]);
    }

    // 其他长字符串 - 保留前20位
    if text.len() > 40 {
        return format!("{}... [truncated {} chars]", &text[..40], text.len() - 40);
    }

    text.to_string()
}

/// 截断长文本
pub fn truncate(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}... [truncated {} chars]", &text[..max_len], text.len() - max_len)
    } else {
        text.to_string()
    }
}

/// 格式化JSON参数，脱敏敏感字段
pub fn mask_json_params(params: &str) -> String {
    // 简单的关键词匹配脱敏
    let sensitive_keys = ["api_key", "token", "secret", "password", "auth"];
    let mut result = params.to_string();

    for key in &sensitive_keys {
        // 匹配 "key": "value" 或 'key': 'value' 格式
        if let Some(pos) = result.find(key) {
            // 找到value的开始位置
            let search_start = pos + key.len();
            if let Some(value_start) = result[search_start..].find('"') {
                let actual_start = search_start + value_start + 1;
                if let Some(value_end) = result[actual_start..].find('"') {
                    let end = actual_start + value_end;
                    if end > actual_start + 8 {
                        let masked = format!("{}...****", &result[actual_start..actual_start + 8]);
                        result.replace_range(actual_start..end, &masked);
                    }
                }
            }
        }
    }

    result
}

/// 记录状态转换
pub fn log_state_transition(
    ctx: &LogContext,
    component: &str,
    from: &str,
    to: &str,
    details: Option<HashMap<String, String>>,
) {
    match details {
        Some(d) => {
            info!(
                trace_id = %ctx.trace_id,
                task_id = ?ctx.task_id,
                fork_id = ?ctx.fork_id,
                agent_name = ?ctx.agent_name,
                app_id = ?ctx.app_id,
                session_id = ?ctx.session_id,
                component = %component,
                from_state = %from,
                to_state = %to,
                details = ?d,
                "[STATE] {}: {} -> {}",
                component, from, to
            );
        }
        None => {
            info!(
                trace_id = %ctx.trace_id,
                task_id = ?ctx.task_id,
                fork_id = ?ctx.fork_id,
                agent_name = ?ctx.agent_name,
                app_id = ?ctx.app_id,
                session_id = ?ctx.session_id,
                component = %component,
                from_state = %from,
                to_state = %to,
                "[STATE] {}: {} -> {}",
                component, from, to
            );
        }
    }
}

/// 记录操作开始
pub fn log_operation_start(ctx: &LogContext, operation: &str, description: Option<&str>) {
    match description {
        Some(desc) => {
            info!(
                trace_id = %ctx.trace_id,
                task_id = ?ctx.task_id,
                fork_id = ?ctx.fork_id,
                agent_name = ?ctx.agent_name,
                app_id = ?ctx.app_id,
                operation = %operation,
                "[START] {}: {}",
                operation, desc
            );
        }
        None => {
            info!(
                trace_id = %ctx.trace_id,
                task_id = ?ctx.task_id,
                fork_id = ?ctx.fork_id,
                agent_name = ?ctx.agent_name,
                app_id = ?ctx.app_id,
                operation = %operation,
                "[START] {}",
                operation
            );
        }
    }
}

/// 记录操作完成
pub fn log_operation_complete(
    ctx: &LogContext,
    operation: &str,
    elapsed: Duration,
    result: &str,
) {
    info!(
        trace_id = %ctx.trace_id,
        task_id = ?ctx.task_id,
        fork_id = ?ctx.fork_id,
        agent_name = ?ctx.agent_name,
        app_id = ?ctx.app_id,
        operation = %operation,
        elapsed_ms = elapsed.as_millis() as u64,
        result = %result,
        "[COMPLETE] {}: {} ({}ms)",
        operation, result, elapsed.as_millis()
    );
}

/// 记录操作错误
pub fn log_operation_error(
    ctx: &LogContext,
    operation: &str,
    error: &str,
    recoverable: bool,
) {
    if recoverable {
        warn!(
            trace_id = %ctx.trace_id,
            task_id = ?ctx.task_id,
            fork_id = ?ctx.fork_id,
            agent_name = ?ctx.agent_name,
            app_id = ?ctx.app_id,
            operation = %operation,
            error = %error,
            "[ERROR] {}: {} (recoverable)",
            operation, error
        );
    } else {
        error!(
            trace_id = %ctx.trace_id,
            task_id = ?ctx.task_id,
            fork_id = ?ctx.fork_id,
            agent_name = ?ctx.agent_name,
            app_id = ?ctx.app_id,
            operation = %operation,
            error = %error,
            "[ERROR] {}: {}",
            operation, error
        );
    }
}

/// 记录Hook事件
pub fn log_hook_event(
    ctx: &LogContext,
    event_type: &str,
    fork_id: &str,
    task_id: Option<&str>,
    details: Option<HashMap<String, String>>,
) {
    match (task_id, details) {
        (Some(tid), Some(d)) => {
            info!(
                trace_id = %ctx.trace_id,
                event_type = %event_type,
                fork_id = %fork_id,
                task_id = %tid,
                app_id = ?ctx.app_id,
                details = ?d,
                "[HOOK] {}: fork={} task={}",
                event_type, fork_id, tid
            );
        }
        (Some(tid), None) => {
            info!(
                trace_id = %ctx.trace_id,
                event_type = %event_type,
                fork_id = %fork_id,
                task_id = %tid,
                app_id = ?ctx.app_id,
                "[HOOK] {}: fork={} task={}",
                event_type, fork_id, tid
            );
        }
        (None, Some(d)) => {
            info!(
                trace_id = %ctx.trace_id,
                event_type = %event_type,
                fork_id = %fork_id,
                app_id = ?ctx.app_id,
                details = ?d,
                "[HOOK] {}: fork={}",
                event_type, fork_id
            );
        }
        (None, None) => {
            info!(
                trace_id = %ctx.trace_id,
                event_type = %event_type,
                fork_id = %fork_id,
                app_id = ?ctx.app_id,
                "[HOOK] {}: fork={}",
                event_type, fork_id
            );
        }
    }
}

/// 记录工具调用
pub fn log_tool_call(
    ctx: &LogContext,
    tool_name: &str,
    tool_input: &str,
    is_error: bool,
) {
    let masked_input = mask_json_params(tool_input);
    let truncated_input = truncate(&masked_input, 500);

    if is_error {
        warn!(
            trace_id = %ctx.trace_id,
            task_id = ?ctx.task_id,
            agent_name = ?ctx.agent_name,
            tool_name = %tool_name,
            input = %truncated_input,
            "[TOOL] {}: ERROR", tool_name
        );
    } else {
        info!(
            trace_id = %ctx.trace_id,
            task_id = ?ctx.task_id,
            agent_name = ?ctx.agent_name,
            tool_name = %tool_name,
            input = %truncated_input,
            "[TOOL] {}: OK", tool_name
        );
    }
}

/// 记录调度决策
pub fn log_scheduling_decision(
    ctx: &LogContext,
    from_agent: &str,
    to_agent: &str,
    reason: &str,
) {
    info!(
        trace_id = %ctx.trace_id,
        task_id = ?ctx.task_id,
        from_agent = %from_agent,
        to_agent = %to_agent,
        reason = %reason,
        "[SCHEDULE] {} -> {}: {}",
        from_agent, to_agent, reason
    );
}

/// 性能计时器
pub struct LogTimer {
    start: Instant,
    operation: String,
}

impl LogTimer {
    /// 创建新的计时器
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            operation: operation.into(),
        }
    }

    /// 记录经过的时间（不结束计时）
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// 结束计时并记录
    pub fn finish(self, ctx: &LogContext, result: &str) -> Duration {
        let elapsed = self.start.elapsed();
        log_operation_complete(ctx, &self.operation, elapsed, result);
        elapsed
    }
}

/// 调试级别的详细日志
pub fn log_debug(ctx: &LogContext, component: &str, message: &str, details: Option<HashMap<String, String>>) {
    match details {
        Some(d) => {
            debug!(
                trace_id = %ctx.trace_id,
                task_id = ?ctx.task_id,
                fork_id = ?ctx.fork_id,
                agent_name = ?ctx.agent_name,
                component = %component,
                details = ?d,
                "[DEBUG] {}: {}", component, message
            );
        }
        None => {
            debug!(
                trace_id = %ctx.trace_id,
                task_id = ?ctx.task_id,
                fork_id = ?ctx.fork_id,
                agent_name = ?ctx.agent_name,
                component = %component,
                "[DEBUG] {}: {}", component, message
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_sensitive() {
        assert_eq!(mask_sensitive("sk-abc1234567890"), "sk-abc12...****");
        assert_eq!(mask_sensitive("normal text"), "normal text");
        assert_eq!(mask_sensitive(""), "");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a long text", 10), "this is a... [truncated 9 chars]");
    }

    #[test]
    fn test_log_context_builder() {
        let ctx = LogContext::new("test-app")
            .with_task_id("task-123")
            .with_agent_name("test-agent");

        assert_eq!(ctx.app_id, Some("test-app".to_string()));
        assert_eq!(ctx.task_id, Some("task-123".to_string()));
        assert_eq!(ctx.agent_name, Some("test-agent".to_string()));
        assert!(!ctx.trace_id.is_empty());
    }
}
