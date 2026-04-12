//! 无真实 LLM 的全链路干跑。
//!
//! **必须**加 `--nocapture` 才能在终端看到分阶段与 Agentic 事件流：
//! `cargo test -p macaca-integration-tests pipeline_dry_run -- --nocapture`

use macaca_integration_tests::pipeline_dry_run::{self, PipelineDryRunConfig};

#[tokio::test]
async fn full_pipeline_dry_run_without_network_llm() {
    let report =
        pipeline_dry_run::run_full_pipeline_dry_run_with_config(PipelineDryRunConfig::verbose())
            .await;
    if report.stages.iter().any(|(_, r)| r.is_err()) {
        eprintln!("{}", report.summary());
    }
    report.assert_all_ok();
}
