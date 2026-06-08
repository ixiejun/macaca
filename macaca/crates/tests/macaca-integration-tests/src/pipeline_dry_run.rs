//! Full pipeline dry-run harness (**Facade** module tree, no real LLM).
//!
//! Exercises persistence → TaskSpace / TaskBoard → dependency + review →
//! `AgenticLoop` + [`crate::scripted_llm::ScriptedLlm`] driving real tools.
//!
//! Recommended trace run:
//! `cargo test -p macaca-integration-tests pipeline_dry_run -- --nocapture`
//!
//! Not covered here: HTTP routes, `ApplicationExecutor::delegate_task`, live drivers —
//! extend via `macaca-web` / `macaca-kernel` scripted-LLM integration tests as needed.
//!
//! # Module tree (P5 iteration 101)
//! - `config` — verbosity toggles (Value Object)
//! - `report` — per-stage outcomes (Memento)
//! - `trace` — stderr + structured tracing (Observer)
//! - `llm` — scripted response fixtures (Test Double)
//! - `store` — ephemeral redb helpers
//! - `fixtures` — neutral harness agent identifiers
//! - `agentic` — traced `AgenticLoop` runner
//! - `stages` — individual stage commands (Command)
//! - `runner` — five-stage orchestration (Template Method)

mod agentic;
mod config;
mod fixtures;
mod llm;
mod report;
mod runner;
mod stages;
mod store;
mod trace;

pub use config::PipelineDryRunConfig;
pub use llm::{response_text, response_with_tools};
pub use report::PipelineReport;
pub use runner::{run_full_pipeline_dry_run, run_full_pipeline_dry_run_with_config};
