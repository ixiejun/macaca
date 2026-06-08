//! Ephemeral persistence helpers for pipeline dry-run stages.
//!
//! Opens a temporary redb database wrapped in [`TodoStore`] for isolated stage runs.

use std::sync::Arc;

use macaca_persist::RedbStore;
use macaca_task::TodoStore;
use tempfile::tempdir;
use tracing::debug;

/// Local composite tool registry type used by agentic stages.
pub type LocalToolSet = macaca_tools::CompositeToolSet;

/// Open a temporary redb-backed [`TodoStore`]; the temp directory is kept alive by the caller.
pub fn open_temp_store() -> Result<(tempfile::TempDir, Arc<TodoStore>), String> {
    let dir = tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("pipeline_dry_run.redb");
    debug!(target: "pipeline_dry_run", path = %path.display(), "opening_temp_redb");
    let store = RedbStore::open(path).map_err(|e| e.to_string())?;
    Ok((dir, Arc::new(TodoStore::new(Arc::new(store)))))
}
