//! Private Component Model adapter boundary.
//!
//! This module is intentionally private to `macaca-runtime-host`.  Public
//! Macaca crates speak only in provider-neutral DTOs, while this Adapter hides
//! how a concrete Component Model engine validates, instantiates, and invokes a
//! guest component.  The default adapter below is a deterministic portable
//! implementation used by tests and governance flows until a concrete engine
//! backend is wired behind the same trait.

use std::collections::BTreeSet;

use macaca_proto::{WasmResourcePolicy, WasmRuntimeErrorKind};

use super::errors::WasmRuntimeHostError;

const WASM_MAGIC: &[u8; 4] = b"\0asm";
const COMPONENT_MARKER: &[u8] = b"macaca:component-model:v1";
const EXPORT_PREFIX: &str = "export=";
const WIT_PREFIX: &str = "wit=";

/// Engine-neutral Adapter trait for Component Model execution.
///
/// The trait is deliberately tiny.  Runtime providers only need validation,
/// instantiation, and export invocation.  A future Wasmtime, WasmEdge, remote
/// daemon, or certified enterprise engine can implement this trait without
/// changing `macaca-proto`, SDK, application framework, kernel, Web, or CLI
/// dependencies.
pub(crate) trait WasmComponentEngineAdapter: Send + Sync + std::fmt::Debug {
    /// Validate bytes and return a provider-private module memento.
    fn validate_component(&self, bytes: &[u8])
        -> Result<WasmComponentModule, WasmRuntimeHostError>;

    /// Instantiate a validated module into an invocation handle.
    fn instantiate(
        &self,
        module: WasmComponentModule,
    ) -> Result<WasmComponentInstance, WasmRuntimeHostError>;
}

/// Deterministic portable adapter used by runtime-host tests.
///
/// The adapter recognizes a minimal provider-neutral fixture envelope: a valid
/// WASM module header plus safe ASCII metadata markers describing WIT package
/// and exported Component Model functions.  It is not exposed as a public ABI
/// and can be replaced by a real engine adapter without changing callers.
#[derive(Debug, Default)]
pub(crate) struct PortableComponentModelAdapter;

impl WasmComponentEngineAdapter for PortableComponentModelAdapter {
    fn validate_component(
        &self,
        bytes: &[u8],
    ) -> Result<WasmComponentModule, WasmRuntimeHostError> {
        if bytes.len() < 8 || &bytes[0..4] != WASM_MAGIC {
            return Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::CompileFailed,
                "WASM component has an invalid module header",
            ));
        }
        if !bytes
            .windows(COMPONENT_MARKER.len())
            .any(|window| window == COMPONENT_MARKER)
        {
            return Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::AbiMismatch,
                "WASM component metadata is missing the Component Model marker",
            ));
        }

        let metadata = String::from_utf8_lossy(bytes);
        let exports = parse_metadata_values(&metadata, EXPORT_PREFIX);
        let wit_packages = parse_metadata_values(&metadata, WIT_PREFIX);
        if exports.is_empty() {
            return Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::AbiMismatch,
                "WASM component metadata declares no exports",
            ));
        }
        if wit_packages.is_empty() {
            return Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::AbiMismatch,
                "WASM component metadata declares no WIT package",
            ));
        }

        Ok(WasmComponentModule {
            exports,
            wit_packages,
            estimated_fuel: bytes.len() as u64,
        })
    }

    fn instantiate(
        &self,
        module: WasmComponentModule,
    ) -> Result<WasmComponentInstance, WasmRuntimeHostError> {
        Ok(WasmComponentInstance { module })
    }
}

/// Provider-private validated component module.
#[derive(Debug, Clone)]
pub(crate) struct WasmComponentModule {
    exports: BTreeSet<String>,
    wit_packages: BTreeSet<String>,
    estimated_fuel: u64,
}

impl WasmComponentModule {
    /// Return whether the component declares an export.
    pub(crate) fn has_export(&self, export_name: &str) -> bool {
        self.exports.contains(export_name)
    }

    /// Return a stable WIT package label for metadata and diagnostics.
    pub(crate) fn wit_label(&self) -> String {
        self.wit_packages
            .iter()
            .next()
            .cloned()
            .unwrap_or_else(|| "unknown".into())
    }

    /// Validate static component resource estimates against the active policy.
    ///
    /// Concrete engines will enforce memory, fuel, and wall-clock limits at
    /// runtime.  This portable adapter still performs the checks it can prove
    /// before instantiation so admission remains fail-closed in tests and local
    /// provider-neutral deployments.
    pub(crate) fn validate_resource_policy(
        &self,
        policy: &WasmResourcePolicy,
    ) -> Result<(), WasmRuntimeHostError> {
        if let Some(limit) = policy.max_fuel {
            if self.estimated_fuel > limit {
                return Err(WasmRuntimeHostError::new(
                    WasmRuntimeErrorKind::ResourceExhausted,
                    "WASM component estimated fuel exceeds the sandbox policy limit",
                ));
            }
        }
        Ok(())
    }
}

/// Provider-private component instance handle.
#[derive(Debug, Clone)]
pub(crate) struct WasmComponentInstance {
    module: WasmComponentModule,
}

impl WasmComponentInstance {
    /// Invoke a declared component export.
    pub(crate) fn invoke_export(&self, export_name: &str) -> Result<(), WasmRuntimeHostError> {
        if self.module.has_export(export_name) {
            Ok(())
        } else {
            Err(WasmRuntimeHostError::new(
                WasmRuntimeErrorKind::InvokeFailed,
                format!("WASM component export '{export_name}' is not available"),
            ))
        }
    }
}

fn parse_metadata_values(metadata: &str, prefix: &str) -> BTreeSet<String> {
    metadata
        .split(|ch| ch == '\0' || ch == '\n' || ch == '\r')
        .filter_map(|part| part.strip_prefix(prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
