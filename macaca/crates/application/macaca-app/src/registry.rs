//! App Registry — discovers and manages applications from standard directories.

use std::collections::HashMap;
use std::path::PathBuf;

use macaca_proto::error::ProtoErrorAdapter;
use macaca_proto::{ApplicationId, MacacaResult};

use crate::loader::AppLoader;
use crate::model::AppManifest;

/// Standard directories where applications are discovered.
pub const STANDARD_APP_DIRS: &[&str] = &[
    "apps",    // Project-local: apps/<app-name>/
    "../apps", // Relative to build directory
];

/// Default application to auto-start if no app is specified.
pub const DEFAULT_APP: &str = "fullstack-autodev";

/// Information about a discovered application (not yet loaded).
#[derive(Debug, Clone)]
pub struct DiscoveredApp {
    /// Application ID from manifest.
    pub id: ApplicationId,
    /// Application name.
    pub name: String,
    /// Path to app directory.
    pub path: PathBuf,
    /// Path to app.yaml manifest.
    pub manifest_path: PathBuf,
    /// Parsed manifest.
    pub manifest: AppManifest,
}

/// Registry for discovering and managing applications.
pub struct AppRegistry {
    /// Map of app ID to discovered app info.
    discovered: HashMap<ApplicationId, DiscoveredApp>,
    /// Map of app name to app ID (for name-based lookup).
    name_to_id: HashMap<String, ApplicationId>,
    /// Directories to scan for apps.
    scan_dirs: Vec<PathBuf>,
}

impl AppRegistry {
    /// Create a new registry with default scan directories.
    pub fn new() -> Self {
        Self {
            discovered: HashMap::new(),
            name_to_id: HashMap::new(),
            scan_dirs: STANDARD_APP_DIRS.iter().map(PathBuf::from).collect(),
        }
    }

    /// Create a registry with custom scan directories.
    pub fn with_dirs(dirs: Vec<PathBuf>) -> Self {
        Self {
            discovered: HashMap::new(),
            name_to_id: HashMap::new(),
            scan_dirs: dirs,
        }
    }

    /// Add a directory to scan for apps.
    pub fn add_scan_dir(&mut self, dir: PathBuf) {
        self.scan_dirs.push(dir);
    }

    /// Build a deterministic app installation directory under the configured
    /// workspace root.
    ///
    /// Industrial deployment expects application discovery to live under
    /// `{workspace.root_dir}/apps` so install and runtime locations are
    /// centrally managed by one configuration root.
    pub fn workspace_apps_dir(workspace_root: impl AsRef<std::path::Path>) -> PathBuf {
        workspace_root.as_ref().join("apps")
    }

    /// Discover all apps from scan directories.
    ///
    /// This scans each directory in `scan_dirs` for subdirectories
    /// containing an `app.yaml` file.
    pub fn discover_apps(&mut self) -> MacacaResult<Vec<DiscoveredApp>> {
        let mut found = Vec::new();

        tracing::info!(
            scan_dirs = ?self.scan_dirs,
            "Scanning configured application directories"
        );

        for scan_dir in &self.scan_dirs {
            if !scan_dir.exists() {
                continue;
            }

            let entries = match std::fs::read_dir(scan_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let manifest_path = path.join("app.yaml");
                if !manifest_path.exists() {
                    continue;
                }

                match AppLoader::load_manifest(&manifest_path) {
                    Ok(manifest) => {
                        let id = manifest.id;
                        let name = manifest.name.clone();

                        let discovered = DiscoveredApp {
                            id,
                            name: name.clone(),
                            path: path.clone(),
                            manifest_path: manifest_path.clone(),
                            manifest,
                        };

                        // Only add if not already discovered
                        if !self.discovered.contains_key(&id) {
                            self.name_to_id.insert(name, id);
                            self.discovered.insert(id, discovered.clone());
                            found.push(discovered);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %manifest_path.display(),
                            error_code = e.code(),
                            error = %e.display_message(),
                            "Failed to load app manifest"
                        );
                    }
                }
            }
        }

        if !found.is_empty() {
            tracing::info!(count = found.len(), "Apps discovered");
        }

        Ok(found)
    }

    /// Get a discovered app by ID.
    pub fn get_app(&self, id: &ApplicationId) -> Option<&DiscoveredApp> {
        self.discovered.get(id)
    }

    /// Get a discovered app by name.
    pub fn get_app_by_name(&self, name: &str) -> Option<&DiscoveredApp> {
        self.name_to_id
            .get(name)
            .and_then(|id| self.discovered.get(id))
    }

    /// List all discovered apps.
    pub fn list_apps(&self) -> Vec<&DiscoveredApp> {
        self.discovered.values().collect()
    }

    /// Get the default app by conventional application name.
    pub fn get_default_app(&self) -> Option<&DiscoveredApp> {
        self.get_app_by_name(DEFAULT_APP)
    }

    /// Find a specific app directory by name.
    /// This is a static method that doesn't require a registry instance.
    pub fn find_app_dir(name: &str) -> Option<PathBuf> {
        // Check environment variable first
        if let Ok(dir) = std::env::var("MACACA_APP_DIR") {
            let path = PathBuf::from(&dir);
            if path.join("app.yaml").exists() {
                return Some(path);
            }
        }

        // Check standard locations
        for base_dir in STANDARD_APP_DIRS {
            let candidate = PathBuf::from(base_dir).join(name);
            if candidate.join("app.yaml").exists() {
                return Some(candidate);
            }
        }

        // Check configured workspace-root apps directory from environment.
        if let Ok(workspace_root) = std::env::var("MACACA_WORKSPACE_ROOT") {
            let workspace_app = Self::workspace_apps_dir(workspace_root).join(name);
            if workspace_app.join("app.yaml").exists() {
                return Some(workspace_app);
            }
        }

        None
    }

    /// Count of discovered apps.
    pub fn app_count(&self) -> usize {
        self.discovered.len()
    }

    /// Clear all discovered apps.
    pub fn clear(&mut self) {
        self.discovered.clear();
        self.name_to_id.clear();
    }

    /// Reload apps (clear and rediscover).
    pub fn reload(&mut self) -> MacacaResult<Vec<DiscoveredApp>> {
        self.clear();
        self.discover_apps()
    }
}

impl Default for AppRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_new_is_empty() {
        let registry = AppRegistry::new();
        assert_eq!(registry.app_count(), 0);
    }

    #[test]
    fn find_app_dir_returns_none_for_nonexistent() {
        let result = AppRegistry::find_app_dir("nonexistent-app-xyz");
        assert!(result.is_none());
    }

    #[test]
    fn discover_apps_from_empty_dir() {
        let temp_dir = std::env::temp_dir().join("macaca_registry_test_empty");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut registry = AppRegistry::with_dirs(vec![temp_dir.clone()]);
        let found = registry.discover_apps().unwrap();
        assert!(found.is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn discover_apps_from_dir_with_app() {
        let temp_dir = std::env::temp_dir().join("macaca_registry_test_app");
        let app_dir = temp_dir.join("test-app");
        std::fs::create_dir_all(&app_dir).unwrap();

        let manifest = r#"
name: test-app
layer: L3Declarative
agents: []
"#;
        std::fs::write(app_dir.join("app.yaml"), manifest).unwrap();

        let mut registry = AppRegistry::with_dirs(vec![temp_dir.clone()]);
        let found = registry.discover_apps().unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "test-app");

        // Verify lookup works
        let by_name = registry.get_app_by_name("test-app");
        assert!(by_name.is_some());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
