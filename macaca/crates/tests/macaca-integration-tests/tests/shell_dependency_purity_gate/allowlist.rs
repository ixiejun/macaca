//! Baseline allowlist for `macaca-web` workspace dependency migration debt (P5 §4.4.4).
//!
//! Terminal state: `macaca-web` depends only on `macaca-proto` and `macaca-sdk` among
//! workspace members. Until composition-root seams are fully absorbed into SDK bridges,
//! these three crates remain as documented migration debt. The gate forbids **new**
//! workspace edges beyond this frozen set.

/// One row of web-shell workspace dependency migration debt.
pub struct WebShellDependencyDebt {
    /// Workspace crate name (`macaca-*`).
    pub crate_name: &'static str,
    /// Why the shell still links this crate directly.
    pub rationale: &'static str,
    /// Replacement boundary once debt is retired.
    pub replacement: &'static str,
}

/// Frozen baseline — any new workspace dependency on `macaca-web` fails CI immediately.
pub const WEB_SHELL_WORKSPACE_DEPENDENCY_DEBT: &[WebShellDependencyDebt] = &[
    WebShellDependencyDebt {
        crate_name: "macaca-app",
        rationale: "Application runtime bootstrap and manifest projection at composition root",
        replacement: "macaca-sdk::application client + runtime-host bootstrap bundle",
    },
    WebShellDependencyDebt {
        crate_name: "macaca-domain-pack-finance",
        rationale: "Optional domain-pack feature wiring at web composition root (SDK bridge blocked by app→sdk→pack→app cycle)",
        replacement: "break app/sdk/pack cycle then macaca-sdk optional domain_pack_finance bridge",
    },
    WebShellDependencyDebt {
        crate_name: "macaca-runtime-host",
        rationale: "Composition-root bootstrap, persist alias, and service provider wiring seam",
        replacement: "macaca-sdk::shell_provider_bridge + public bootstrap facade",
    },
];

/// Workspace crate names permitted at terminal state for presentation shells.
pub const PERMITTED_SHELL_WORKSPACE_DEPS: &[&str] = &["macaca-proto", "macaca-sdk"];
