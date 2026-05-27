//! Agent OS CLI entry point.

#![deny(deprecated)]

use clap::{Args, Parser, Subcommand};
use macaca_cli::command_handlers::{
    AgentsCommandHandler, CliCommandHandler, PluginListCommandHandler, RunCommandHandler,
    SkillCurationRunCommandHandler, SkillLifecycleCommandHandler,
    SkillOperationsSnapshotCommandHandler, SkillProposalDecisionCommandHandler,
    SkillRollbackCommandHandler, StatusCommandHandler, ToolOperationsSnapshotCommandHandler,
    VersionCommandHandler, WebCommandHandler,
};
use macaca_cli::skill_operations::{
    SkillCliEvidenceRefs, SkillCliLifecycleAction, SkillCliRuntimeTarget,
};
use macaca_cli::tool_operations::ToolCliRuntimeTarget;
use macaca_proto::config::MacacaConfig;

/// Agent OS — autonomous agent orchestration platform.
#[derive(Parser)]
#[command(name = "macaca", version, about = "Macaca Agent OS CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands.
#[derive(Subcommand)]
enum Commands {
    /// Start the Agent OS kernel and gateway adapters.
    Run,
    /// List all registered agents.
    Agents,
    /// Show system status.
    Status,
    /// Plugin Control management commands.
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
    /// Skill governance and curation operations.
    Skill {
        #[command(flatten)]
        target: SkillRuntimeArgs,
        #[command(subcommand)]
        command: SkillCommands,
    },
    /// Tool Capability Plane diagnostics.
    Tool {
        #[command(flatten)]
        target: ToolRuntimeArgs,
        #[command(subcommand)]
        command: ToolCommands,
    },
    /// Print version information.
    Version,
    /// Start the web UI server.
    Web {
        /// Port to listen on.
        #[arg(long, default_value_t = 3001)]
        port: u16,
    },
}

/// Plugin management subcommands.
#[derive(Subcommand)]
enum PluginCommands {
    /// List plugins through Plugin Control Service.
    List,
}

/// Optional live Web/runtime target for Skill operations.
///
/// These flags keep CLI as a terminal Adapter.  Supplying `--app-id` tells CLI
/// to call the public Web API facade for the already-running runtime.  Omitting
/// it preserves the structured SDK unavailable diagnostic path.
#[derive(Args, Clone, Default)]
struct SkillRuntimeArgs {
    #[arg(long)]
    app_id: Option<String>,
    #[arg(long)]
    api_base: Option<String>,
}

/// Optional live Web/runtime target for Tool diagnostics.
#[derive(Args, Clone, Default)]
struct ToolRuntimeArgs {
    #[arg(long)]
    app_id: Option<String>,
    #[arg(long)]
    api_base: Option<String>,
}

/// Tool diagnostics subcommands.
#[derive(Subcommand)]
enum ToolCommands {
    /// Print bounded Tool plan, provider health, and audit replay state.
    Operations,
}

/// Skill operations subcommands.
#[derive(Subcommand)]
enum SkillCommands {
    /// Print bounded Skill governance and proposal state.
    Operations,
    /// Run deterministic curation analysis.
    CurationRun {
        #[arg(long, default_value_t = 30)]
        stale_after_days: i64,
        #[arg(long, default_value_t = 1)]
        narrow_use_threshold: u64,
    },
    /// Apply an approval-gated curation run.
    CurationApply {
        #[arg(long, default_value_t = 30)]
        stale_after_days: i64,
        #[arg(long, default_value_t = 1)]
        narrow_use_threshold: u64,
        #[command(flatten)]
        refs: SkillEvidenceArgs,
    },
    /// Forward a lifecycle action through the Skill service facade.
    Lifecycle {
        #[command(subcommand)]
        action: SkillLifecycleCommands,
    },
    /// Restore a curation rollback memento.
    Rollback {
        rollback_ref: String,
        #[command(flatten)]
        refs: SkillEvidenceArgs,
    },
    /// Promote or reject a draft Skill proposal.
    Proposal {
        #[command(subcommand)]
        command: SkillProposalCommands,
    },
}

/// CLI lifecycle actions exposed as transport labels only.
#[derive(Subcommand)]
enum SkillLifecycleCommands {
    Pin(SkillTargetArgs),
    Unpin(SkillTargetArgs),
    Archive(SkillTargetArgs),
    Restore(SkillTargetArgs),
    Quarantine(SkillTargetArgs),
    ReleaseQuarantine(SkillTargetArgs),
    Reject(SkillTargetArgs),
}

/// Proposal lifecycle actions.
#[derive(Subcommand)]
enum SkillProposalCommands {
    Promote(SkillProposalArgs),
    Reject(SkillProposalArgs),
}

/// Shared target arguments for lifecycle operations.
#[derive(Args)]
struct SkillTargetArgs {
    skill_id: String,
    #[command(flatten)]
    refs: SkillEvidenceArgs,
}

/// Shared target arguments for proposal operations.
#[derive(Args)]
struct SkillProposalArgs {
    proposal_id: String,
    #[command(flatten)]
    refs: SkillEvidenceArgs,
}

/// Evidence refs passed through to SDK DTOs without local policy decisions.
#[derive(Args, Clone)]
struct SkillEvidenceArgs {
    #[arg(long)]
    reason: Option<String>,
    #[arg(long)]
    evidence_ref: Option<String>,
    #[arg(long)]
    policy_ref: Option<String>,
    #[arg(long)]
    approval_ref: Option<String>,
}

#[tokio::main]
async fn main() {
    // Load configuration
    let config = MacacaConfig::load_default();

    // Initialize logging with file output
    if let Err(e) = macaca_cli::logging::init_logging(
        &config.observability.log_file,
        &config.observability.log_level,
    ) {
        eprintln!("Failed to initialize logging: {e}");
        std::process::exit(1);
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run => RunCommandHandler.run().await,
        Commands::Agents => AgentsCommandHandler.run().await,
        Commands::Status => StatusCommandHandler.run().await,
        Commands::Plugin { command } => match command {
            PluginCommands::List => PluginListCommandHandler.run().await,
        },
        Commands::Skill { target, command } => run_skill_command(target.into(), command).await,
        Commands::Tool { target, command } => run_tool_command(target.into(), command).await,
        Commands::Version => {
            VersionCommandHandler::new(env!("CARGO_PKG_VERSION"))
                .run()
                .await
        }
        Commands::Web { port } => WebCommandHandler::new(port).run().await,
    };

    if let Err(e) = result {
        tracing::error!(error = %e, "Command failed");
        std::process::exit(1);
    }
}

async fn run_tool_command(
    target: ToolCliRuntimeTarget,
    command: ToolCommands,
) -> macaca_proto::error::MacacaResult<()> {
    match command {
        ToolCommands::Operations => {
            ToolOperationsSnapshotCommandHandler::new(target)
                .run()
                .await
        }
    }
}

async fn run_skill_command(
    target: SkillCliRuntimeTarget,
    command: SkillCommands,
) -> macaca_proto::error::MacacaResult<()> {
    match command {
        SkillCommands::Operations => {
            SkillOperationsSnapshotCommandHandler::new(target)
                .run()
                .await
        }
        SkillCommands::CurationRun {
            stale_after_days,
            narrow_use_threshold,
        } => {
            SkillCurationRunCommandHandler::new(
                target,
                true,
                stale_after_days,
                narrow_use_threshold,
                SkillCliEvidenceRefs::default(),
            )
            .run()
            .await
        }
        SkillCommands::CurationApply {
            stale_after_days,
            narrow_use_threshold,
            refs,
        } => {
            SkillCurationRunCommandHandler::new(
                target,
                false,
                stale_after_days,
                narrow_use_threshold,
                refs.into(),
            )
            .run()
            .await
        }
        SkillCommands::Lifecycle { action } => run_skill_lifecycle_command(target, action).await,
        SkillCommands::Rollback { rollback_ref, refs } => {
            SkillRollbackCommandHandler::new(target, rollback_ref, refs.into())
                .run()
                .await
        }
        SkillCommands::Proposal { command } => run_skill_proposal_command(target, command).await,
    }
}

async fn run_skill_lifecycle_command(
    runtime_target: SkillCliRuntimeTarget,
    action: SkillLifecycleCommands,
) -> macaca_proto::error::MacacaResult<()> {
    let (action, skill_target) = match action {
        SkillLifecycleCommands::Pin(target) => (SkillCliLifecycleAction::Pin, target),
        SkillLifecycleCommands::Unpin(target) => (SkillCliLifecycleAction::Unpin, target),
        SkillLifecycleCommands::Archive(target) => (SkillCliLifecycleAction::Archive, target),
        SkillLifecycleCommands::Restore(target) => (SkillCliLifecycleAction::Restore, target),
        SkillLifecycleCommands::Quarantine(target) => (SkillCliLifecycleAction::Quarantine, target),
        SkillLifecycleCommands::ReleaseQuarantine(target) => {
            (SkillCliLifecycleAction::ReleaseQuarantine, target)
        }
        SkillLifecycleCommands::Reject(target) => (SkillCliLifecycleAction::Reject, target),
    };
    SkillLifecycleCommandHandler::new(
        runtime_target,
        action,
        skill_target.skill_id,
        skill_target.refs.into(),
    )
    .run()
    .await
}

async fn run_skill_proposal_command(
    target: SkillCliRuntimeTarget,
    command: SkillProposalCommands,
) -> macaca_proto::error::MacacaResult<()> {
    match command {
        SkillProposalCommands::Promote(args) => {
            SkillProposalDecisionCommandHandler::new(
                target,
                args.proposal_id,
                true,
                args.refs.into(),
            )
            .run()
            .await
        }
        SkillProposalCommands::Reject(args) => {
            SkillProposalDecisionCommandHandler::new(
                target,
                args.proposal_id,
                false,
                args.refs.into(),
            )
            .run()
            .await
        }
    }
}

impl From<SkillRuntimeArgs> for SkillCliRuntimeTarget {
    fn from(args: SkillRuntimeArgs) -> Self {
        Self {
            app_id: args.app_id,
            api_base: args.api_base,
        }
    }
}

impl From<ToolRuntimeArgs> for ToolCliRuntimeTarget {
    fn from(args: ToolRuntimeArgs) -> Self {
        Self {
            app_id: args.app_id,
            api_base: args.api_base,
        }
    }
}

impl From<SkillEvidenceArgs> for SkillCliEvidenceRefs {
    fn from(value: SkillEvidenceArgs) -> Self {
        Self {
            reason: value.reason,
            evidence_ref: value.evidence_ref,
            policy_ref: value.policy_ref,
            approval_ref: value.approval_ref,
        }
    }
}
