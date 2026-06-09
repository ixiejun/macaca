//! Agent execution wiring tests for the Web shell.
//!
//! Production `service.agent_execution` semantics are owned by runtime-host
//! [`macaca_sdk::runtime_host::ComposedAgentExecutionBackend`].  Web injects host
//! and framework adapters through [`crate::web_agent_execution_adapters`].

#[cfg(test)]
mod tests;
