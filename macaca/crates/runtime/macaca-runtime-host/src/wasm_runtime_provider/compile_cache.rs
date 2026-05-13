//! In-memory compiled artifact cache.
//!
//! The cache uses provider-neutral cache keys from `macaca-proto` and stores
//! only private compiled module mementos.  This keeps cache behavior auditable
//! while avoiding public exposure of engine handles or raw WASM bytes.

use std::collections::{hash_map::DefaultHasher, BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use macaca_proto::{
    WasmCompiledArtifactCacheKey, WasmCompiledArtifactCacheReport, WasmCompiledArtifactCacheState,
    WasmRuntimeSessionRequest,
};
use tracing::debug;

use super::engine_adapter::{CompiledWasmModule, InProcessWasmEngineAdapter};
use super::errors::WasmRuntimeHostError;

/// Thread-safe cache shared by provider clones.
#[derive(Debug, Default)]
pub(crate) struct InMemoryCompiledArtifactCache {
    modules: Mutex<HashMap<WasmCompiledArtifactCacheKey, Arc<CompiledWasmModule>>>,
}

impl InMemoryCompiledArtifactCache {
    /// Compile or reuse a module and return an auditable cache report.
    pub(crate) fn get_or_compile(
        &self,
        request: &WasmRuntimeSessionRequest,
        bytes: &[u8],
        adapter: &InProcessWasmEngineAdapter,
    ) -> Result<(Arc<CompiledWasmModule>, WasmCompiledArtifactCacheReport), WasmRuntimeHostError>
    {
        let key = cache_key(request, bytes);
        if let Some(module) = self
            .modules
            .lock()
            .expect("cache mutex poisoned")
            .get(&key)
            .cloned()
        {
            debug!(
                artifact_digest_prefix = %digest_prefix(&key.digest_value),
                cache_state = "hit",
                "WASM compiled artifact cache hit"
            );
            return Ok((
                module,
                WasmCompiledArtifactCacheReport::new(
                    key,
                    WasmCompiledArtifactCacheState::Hit,
                    "compiled artifact reused",
                ),
            ));
        }
        let module = Arc::new(adapter.compile(bytes)?);
        self.modules
            .lock()
            .expect("cache mutex poisoned")
            .insert(key.clone(), Arc::clone(&module));
        debug!(
            artifact_digest_prefix = %digest_prefix(&key.digest_value),
            cache_state = "stored",
            "WASM compiled artifact stored in cache"
        );
        Ok((
            module,
            WasmCompiledArtifactCacheReport::new(
                key,
                WasmCompiledArtifactCacheState::Stored,
                "compiled artifact stored",
            ),
        ))
    }
}

fn cache_key(request: &WasmRuntimeSessionRequest, bytes: &[u8]) -> WasmCompiledArtifactCacheKey {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "capability_fingerprint".into(),
        "default-in-process-core-wasm-v0".into(),
    );
    metadata.insert(
        "profile_fingerprint".into(),
        format!(
            "{}:{:?}:{:?}",
            request.profile.runtime_kind,
            request.profile.compile_mode,
            request.profile.instantiation_mode
        ),
    );
    let digest_algorithm = request
        .metadata
        .get("artifact.digest.algorithm")
        .cloned()
        .unwrap_or_else(|| "stable-hash".into());
    let digest_value = request
        .metadata
        .get("artifact.digest.value")
        .cloned()
        .unwrap_or_else(|| stable_hash(bytes));
    WasmCompiledArtifactCacheKey::new(
        digest_algorithm,
        digest_value,
        request.profile.abi_version.to_string(),
        &metadata,
    )
}

fn stable_hash(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn digest_prefix(value: &str) -> String {
    value.chars().take(12).collect()
}
