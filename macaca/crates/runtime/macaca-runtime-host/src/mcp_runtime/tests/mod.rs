//! Contract tests for MCP runtime facade, manager, and policy modules.
//!
//! Tests are partitioned by concern (fixtures, catalog/config, facade lifecycle,
//! service-owned invocation) so each file stays within the OS layer 500-line gate.

mod catalog;
mod facade_lifecycle;
mod fixtures;
mod invocation;
