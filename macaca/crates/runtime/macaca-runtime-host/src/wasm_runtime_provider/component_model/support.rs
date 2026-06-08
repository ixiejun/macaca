//! Artifact loading and component-model dispatch classification helpers.
//!
//! **Pattern:** Strategy support — pure functions that classify dispatch paths
//! and load artifact bytes without coupling to session state.

use std::fs;
use std::path::{Path, PathBuf};

use macaca_proto::{
    ApplicationHostCommand, ApplicationImport, WasmRuntimeErrorKind, WasmRuntimeSessionRequest,
};

use crate::wasm_runtime_provider::errors::WasmRuntimeHostError;

/// Returns `true` when the command targets a component export invoke path
/// (as opposed to a direct host-import dispatch).
pub(super) fn is_component_export_invoke(command: &ApplicationHostCommand) -> bool {
    command.metadata.contains_key("wasm.export")
        || matches!(
            &command.import,
            ApplicationImport::Custom(value) if value == "macaca:wasm/invoke"
        )
}

/// Loads raw WASM bytes from the artifact reference embedded in `request`.
///
/// Logs the resolved path at debug level for traceability during artifact
/// resolution failures.
pub(super) fn load_artifact_bytes(
    request: &WasmRuntimeSessionRequest,
) -> Result<Vec<u8>, WasmRuntimeHostError> {
    let path = artifact_path(request.artifact.as_str())?;
    tracing::debug!(
        target: "macaca.runtime.component_model",
        artifact_path = %path.display(),
        "loading component-model artifact bytes"
    );
    fs::read(&path).map_err(|error| {
        WasmRuntimeHostError::new(
            WasmRuntimeErrorKind::ArtifactLoadFailed,
            format!("WASM component artifact could not be loaded: {error}"),
        )
    })
}

/// Resolves an artifact reference string into a filesystem path.
///
/// Accepts bare paths and `file://` URIs.  Rejects empty or non-resolving
/// references so sandbox admission never proceeds with an ambiguous artifact.
pub(super) fn artifact_path(reference: &str) -> Result<PathBuf, WasmRuntimeHostError> {
    let trimmed = reference.trim();
    let path = trimmed.strip_prefix("file://").unwrap_or(trimmed);
    if path.is_empty() {
        return Err(WasmRuntimeHostError::new(
            WasmRuntimeErrorKind::ArtifactLoadFailed,
            "WASM component artifact reference is empty",
        ));
    }
    let candidate = Path::new(path);
    if candidate.components().next().is_none() {
        return Err(WasmRuntimeHostError::new(
            WasmRuntimeErrorKind::ArtifactLoadFailed,
            "WASM component artifact reference does not resolve to a file",
        ));
    }
    Ok(candidate.to_path_buf())
}

/// Computes a stable digest string for the component artifact bytes.
///
/// The digest is used for lifecycle mementos and audit metadata; it is not a
/// cryptographic hash but a deterministic fingerprint for portable components.
pub(super) fn component_digest(bytes: &[u8]) -> String {
    let sum = bytes.iter().fold(0u64, |accumulator, byte| {
        accumulator.wrapping_add(*byte as u64)
    });
    format!("portable-component-{sum:016x}")
}
