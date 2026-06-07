//! Provider-neutral persistence ports owned by the kernel.
//!
//! The kernel must describe *what* durable behavior it needs without naming
//! any concrete database crate.  These ports use the Repository pattern for
//! storage, the Memento pattern for replayable state snapshots, and the Null
//! Object pattern for callers that intentionally run without durability.

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use tracing::{debug, warn};

/// Generic key-value repository used by kernel audit and execution recovery.
///
/// The key namespace is deliberately string-based because audit, queues, and
/// fork recovery already own their stable prefixes.  Concrete stores remain
/// outside the kernel and only need to implement these four primitive
/// operations.  This keeps persistence replaceable while preserving traceable
/// kernel recovery semantics.
#[async_trait]
pub trait KernelPersistencePort: Send + Sync {
    /// Load one binary memento by key.
    async fn get(&self, key: &str) -> MacacaResult<Option<Vec<u8>>>;

    /// Upsert one binary memento by key.
    async fn set(&self, key: &str, value: &[u8]) -> MacacaResult<()>;

    /// Delete one memento by key.  Missing keys are expected to be harmless.
    async fn delete(&self, key: &str) -> MacacaResult<()>;

    /// List keys under one prefix so restart recovery can replay stored items.
    async fn list_keys(&self, prefix: &str) -> MacacaResult<Vec<String>>;

    /// Human-readable backend label for diagnostics and audit logs.
    fn backend_name(&self) -> &'static str {
        "kernel-persistence-port"
    }
}

/// Null Object persistence port for intentionally non-durable construction.
///
/// Reads return empty data and writes are acknowledged with a warning.  This is
/// useful for service-client-only or test paths that do not need persistence,
/// while still making the missing durable backend visible in logs.
#[derive(Debug, Default)]
pub struct UnavailableKernelPersistencePort;

#[async_trait]
impl KernelPersistencePort for UnavailableKernelPersistencePort {
    async fn get(&self, key: &str) -> MacacaResult<Option<Vec<u8>>> {
        debug!(
            key,
            "kernel persistence get skipped because no durable backend is configured"
        );
        Ok(None)
    }

    async fn set(&self, key: &str, _value: &[u8]) -> MacacaResult<()> {
        warn!(
            key,
            "kernel persistence set skipped because no durable backend is configured"
        );
        Ok(())
    }

    async fn delete(&self, key: &str) -> MacacaResult<()> {
        debug!(
            key,
            "kernel persistence delete skipped because no durable backend is configured"
        );
        Ok(())
    }

    async fn list_keys(&self, prefix: &str) -> MacacaResult<Vec<String>> {
        debug!(
            prefix,
            "kernel persistence list skipped because no durable backend is configured"
        );
        Ok(Vec::new())
    }

    fn backend_name(&self) -> &'static str {
        "unavailable-kernel-persistence"
    }
}
