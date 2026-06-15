//! Baseline exception inventory for `macaca-web` workspace dependency debt (P5 §4.4.4).
//!
//! Terminal state (§9.4): `macaca-web` depends only on `macaca-proto` and `macaca-sdk`
//! among workspace members. The frozen debt list is empty — any new direct workspace
//! edge on `macaca-web` fails CI immediately.

/// One row of web-shell workspace dependency debt.
pub struct WebShellDependencyDebt {
    /// Workspace crate name (`macaca-*`).
    pub crate_name: &'static str,
    /// Why the shell still links this crate directly.
    pub rationale: &'static str,
    /// Replacement boundary once debt is retired.
    pub replacement: &'static str,
}

/// Frozen baseline — empty at §9.4 terminal state (Iteration 125).
pub const WEB_SHELL_WORKSPACE_DEPENDENCY_DEBT: &[WebShellDependencyDebt] = &[];

/// Workspace crate names permitted at terminal state for presentation shells.
pub const PERMITTED_SHELL_WORKSPACE_DEPS: &[&str] = &["macaca-proto", "macaca-sdk"];
