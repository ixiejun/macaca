//! Command handler primitives for CLI subcommand dispatch.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use async_trait::async_trait;
use macaca_proto::error::{MacacaError, MacacaResult};
use tokio::process::Command;

use crate::commands;
use crate::skill_operations::{
    self, SkillCliEvidenceRefs, SkillCliLifecycleAction, SkillCliRuntimeTarget,
};
use crate::tool_operations::{self, ToolCliRuntimeTarget};
use crate::workbench_operations::{self, WorkbenchCliRuntimeTarget};

/// Canonical command execution boundary for CLI subcommands.
#[async_trait(?Send)]
pub trait CliCommandHandler {
    /// Execute the selected CLI command.
    async fn run(&self) -> MacacaResult<()>;
}

/// Handler for the `run` command.
#[derive(Debug, Default)]
pub struct RunCommandHandler;

#[async_trait(?Send)]
impl CliCommandHandler for RunCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        commands::execute_run_kernel().await
    }
}

/// Handler for the `agents` command.
#[derive(Debug, Default)]
pub struct AgentsCommandHandler;

#[async_trait(?Send)]
impl CliCommandHandler for AgentsCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        commands::execute_list_agents().await
    }
}

/// Handler for the `status` command.
#[derive(Debug, Default)]
pub struct StatusCommandHandler;

#[async_trait(?Send)]
impl CliCommandHandler for StatusCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        commands::execute_show_status().await
    }
}

/// Handler for read-only plugin management commands.
///
/// CLI remains a thin shell: the handler delegates to SDK Plugin Control
/// client functions and never reads plugin repositories or runtime-host state
/// directly.
#[derive(Debug, Default)]
pub struct PluginListCommandHandler;

#[async_trait(?Send)]
impl CliCommandHandler for PluginListCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        commands::execute_list_plugins().await
    }
}

/// Handler for printing bounded Tool Capability Plane diagnostics.
#[derive(Debug, Default)]
pub struct ToolOperationsSnapshotCommandHandler {
    target: ToolCliRuntimeTarget,
}

impl ToolOperationsSnapshotCommandHandler {
    pub fn new(target: ToolCliRuntimeTarget) -> Self {
        Self { target }
    }
}

#[async_trait(?Send)]
impl CliCommandHandler for ToolOperationsSnapshotCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        tool_operations::execute_tool_operations_snapshot(self.target.clone()).await
    }
}

/// Handler for printing bounded workbench-family diagnostics.
#[derive(Debug, Default)]
pub struct WorkbenchOperationsSnapshotCommandHandler {
    target: WorkbenchCliRuntimeTarget,
}

impl WorkbenchOperationsSnapshotCommandHandler {
    pub fn new(target: WorkbenchCliRuntimeTarget) -> Self {
        Self { target }
    }
}

#[async_trait(?Send)]
impl CliCommandHandler for WorkbenchOperationsSnapshotCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        workbench_operations::execute_workbench_operations_snapshot(self.target.clone()).await
    }
}

/// Handler for printing bounded Skill governance operations state.
#[derive(Debug, Default)]
pub struct SkillOperationsSnapshotCommandHandler {
    target: SkillCliRuntimeTarget,
}

impl SkillOperationsSnapshotCommandHandler {
    pub fn new(target: SkillCliRuntimeTarget) -> Self {
        Self { target }
    }
}

#[async_trait(?Send)]
impl CliCommandHandler for SkillOperationsSnapshotCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        skill_operations::execute_skill_operations_snapshot(self.target.clone()).await
    }
}

/// Handler for deterministic or approval-gated Skill curation runs.
#[derive(Debug)]
pub struct SkillCurationRunCommandHandler {
    target: SkillCliRuntimeTarget,
    dry_run: bool,
    stale_after_days: i64,
    narrow_use_threshold: u64,
    refs: SkillCliEvidenceRefs,
}

impl SkillCurationRunCommandHandler {
    pub fn new(
        target: SkillCliRuntimeTarget,
        dry_run: bool,
        stale_after_days: i64,
        narrow_use_threshold: u64,
        refs: SkillCliEvidenceRefs,
    ) -> Self {
        Self {
            target,
            dry_run,
            stale_after_days,
            narrow_use_threshold,
            refs,
        }
    }
}

#[async_trait(?Send)]
impl CliCommandHandler for SkillCurationRunCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        skill_operations::execute_skill_curation_run(
            self.target.clone(),
            self.dry_run,
            self.stale_after_days,
            self.narrow_use_threshold,
            self.refs.clone(),
        )
        .await
    }
}

/// Handler for one Skill lifecycle operation.
#[derive(Debug)]
pub struct SkillLifecycleCommandHandler {
    target: SkillCliRuntimeTarget,
    action: SkillCliLifecycleAction,
    skill_id: String,
    refs: SkillCliEvidenceRefs,
}

impl SkillLifecycleCommandHandler {
    pub fn new(
        target: SkillCliRuntimeTarget,
        action: SkillCliLifecycleAction,
        skill_id: String,
        refs: SkillCliEvidenceRefs,
    ) -> Self {
        Self {
            target,
            action,
            skill_id,
            refs,
        }
    }
}

#[async_trait(?Send)]
impl CliCommandHandler for SkillLifecycleCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        skill_operations::execute_skill_lifecycle(
            self.target.clone(),
            self.action,
            self.skill_id.clone(),
            self.refs.clone(),
        )
        .await
    }
}

/// Handler for restoring a Skill curation rollback memento.
#[derive(Debug)]
pub struct SkillRollbackCommandHandler {
    target: SkillCliRuntimeTarget,
    rollback_ref: String,
    refs: SkillCliEvidenceRefs,
}

impl SkillRollbackCommandHandler {
    pub fn new(
        target: SkillCliRuntimeTarget,
        rollback_ref: String,
        refs: SkillCliEvidenceRefs,
    ) -> Self {
        Self {
            target,
            rollback_ref,
            refs,
        }
    }
}

#[async_trait(?Send)]
impl CliCommandHandler for SkillRollbackCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        skill_operations::execute_skill_rollback(
            self.target.clone(),
            self.rollback_ref.clone(),
            self.refs.clone(),
        )
        .await
    }
}

/// Handler for proposal promotion or rejection.
#[derive(Debug)]
pub struct SkillProposalDecisionCommandHandler {
    target: SkillCliRuntimeTarget,
    proposal_id: String,
    promote: bool,
    refs: SkillCliEvidenceRefs,
}

impl SkillProposalDecisionCommandHandler {
    pub fn new(
        target: SkillCliRuntimeTarget,
        proposal_id: String,
        promote: bool,
        refs: SkillCliEvidenceRefs,
    ) -> Self {
        Self {
            target,
            proposal_id,
            promote,
            refs,
        }
    }
}

#[async_trait(?Send)]
impl CliCommandHandler for SkillProposalDecisionCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        skill_operations::execute_skill_proposal_decision(
            self.target.clone(),
            self.proposal_id.clone(),
            self.promote,
            self.refs.clone(),
        )
        .await
    }
}

/// Handler for the `version` command.
#[derive(Debug)]
pub struct VersionCommandHandler {
    version: &'static str,
}

impl VersionCommandHandler {
    /// Build a version command handler.
    pub fn new(version: &'static str) -> Self {
        Self { version }
    }
}

#[async_trait(?Send)]
impl CliCommandHandler for VersionCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        println!("Macaca Agent OS v{}", self.version);
        Ok(())
    }
}

/// Handler for the `web` command.
///
/// This handler is intentionally a terminal adapter and process-lifecycle
/// boundary only. It selects the listen port, notifies the service manager, and
/// starts the standalone Web shell process without linking CLI to Web internals.
#[derive(Debug)]
pub struct WebCommandHandler {
    port: u16,
}

impl WebCommandHandler {
    /// Build a web command handler for the given port.
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

#[async_trait(?Send)]
impl CliCommandHandler for WebCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        tracing::info!(
            port = self.port,
            "cli web command launching standalone web shell process"
        );
        notify_systemd_ready();
        WebServerProcessLauncher::default().serve(self.port).await
    }
}

/// Process launcher for the Web shell binary.
///
/// The Strategy is deliberately small: prefer an installed sibling binary for
/// packaged deployments, then fall back to `cargo run` in development
/// workspaces. CLI owns only process supervision; Web owns bootstrap semantics.
#[derive(Debug, Default)]
struct WebServerProcessLauncher;

impl WebServerProcessLauncher {
    async fn serve(&self, port: u16) -> MacacaResult<()> {
        let mut command = self.command(port)?;
        let status = command.status().await?;
        if status.success() {
            return Ok(());
        }
        Err(MacacaError::Config(format!(
            "macaca-web-server exited with status {status}"
        )))
    }

    fn command(&self, port: u16) -> MacacaResult<Command> {
        if let Some(binary) = sibling_web_server_binary()? {
            let mut command = Command::new(binary);
            command.arg("--port").arg(port.to_string());
            inherit_stdio(&mut command);
            return Ok(command);
        }

        let workspace = workspace_root()?;
        let mut command = Command::new("cargo");
        command.current_dir(workspace).args([
            "run",
            "-p",
            "macaca-host-composition",
            "--bin",
            "macaca-web-server",
            "--",
            "--port",
            &port.to_string(),
        ]);
        inherit_stdio(&mut command);
        Ok(command)
    }
}

fn inherit_stdio(command: &mut Command) {
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
}

fn sibling_web_server_binary() -> MacacaResult<Option<PathBuf>> {
    let current = std::env::current_exe()?;
    let Some(parent) = current.parent() else {
        return Ok(None);
    };
    let candidate = parent.join(format!("macaca-web-server{}", std::env::consts::EXE_SUFFIX));
    Ok(candidate.is_file().then_some(candidate))
}

fn workspace_root() -> MacacaResult<PathBuf> {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return Ok(ancestor.to_path_buf());
            }
        }
    }
    Err(MacacaError::Config(
        "failed to locate Cargo workspace for macaca-web-server fallback".into(),
    ))
}

#[cfg(feature = "systemd")]
fn notify_systemd_ready() {
    let _ = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]);

    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Watchdog]);
        }
    });
}

#[cfg(not(feature = "systemd"))]
fn notify_systemd_ready() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn agents_handler_runs() {
        AgentsCommandHandler.run().await.unwrap();
    }

    #[tokio::test]
    async fn status_handler_runs() {
        StatusCommandHandler.run().await.unwrap();
    }

    #[tokio::test]
    async fn version_handler_runs() {
        VersionCommandHandler::new("test-version")
            .run()
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn plugin_list_handler_runs() {
        PluginListCommandHandler.run().await.unwrap();
    }

    #[test]
    fn web_command_uses_only_public_server_start_seam() {
        let source = include_str!("command_handlers.rs");
        assert!(
            source.contains("macaca-web-server"),
            "CLI web command must launch the standalone Web shell process"
        );
        assert!(
            !source.contains(&format!("{}{}", "macaca_", "web::")),
            "CLI web command must not link against Web crate internals"
        );
    }
}
