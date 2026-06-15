//! Provider-neutral Memory service contract re-exports.
//!
//! The canonical service command/result DTOs live in `macaca-proto`. This module
//! keeps existing Memory-service imports stable while ensuring SDK and
//! runtime-host use the same protocol-owned contract types.

use macaca_proto::{MacacaResult, MemoryScope};

use crate::vector_topology::{DefaultVectorTopologyResolver, VectorTopologyResolver};

pub use macaca_proto::{
    MemoryForgetCommand, MemoryGetCommand, MemoryGetResult, MemoryPolicyHints,
    MemoryPrefetchCommand, MemoryRecallCommand, MemoryRecallResult, MemoryRememberCommand,
    MemoryRememberResult, MemoryServiceSnapshot, MemoryServiceSnapshotCommand, MemoryStatusCommand,
    MemoryTopologyLabels, MEMORY_FORGET_COMMAND, MEMORY_GET_COMMAND, MEMORY_PREFETCH_COMMAND,
    MEMORY_RECALL_COMMAND, MEMORY_REMEMBER_COMMAND, MEMORY_SERVICE_ID, MEMORY_SNAPSHOT_COMMAND,
    MEMORY_STATUS_COMMAND,
};

/// Resolve provider-neutral topology labels from a scope.
pub fn topology_labels_for_scope(scope: &MemoryScope) -> MacacaResult<MemoryTopologyLabels> {
    let topology = DefaultVectorTopologyResolver.resolve(scope)?;
    Ok(MemoryTopologyLabels {
        application_namespace: topology.database,
        agent_collection: topology.collection,
        shared_collection: topology.shared_collection,
    })
}
