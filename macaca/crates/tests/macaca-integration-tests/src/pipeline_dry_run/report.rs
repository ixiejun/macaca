//! Per-stage outcome aggregation for pipeline dry-run diagnostics.
//!
//! **Design pattern:** Memento — captures pass/fail state per stage so callers can
//! assert on partial or full runs and print a human-readable summary on failure.

/// Aggregated stage results for a single dry-run invocation.
#[derive(Debug, Clone)]
pub struct PipelineReport {
    /// Ordered list of `(stage_name, outcome)` pairs recorded during the run.
    pub stages: Vec<(String, Result<(), String>)>,
}

impl PipelineReport {
    /// Panics when any stage failed; intended for test assertions.
    pub fn assert_all_ok(&self) {
        let failures: Vec<_> = self
            .stages
            .iter()
            .filter_map(|(n, r)| r.as_ref().err().map(|e| format!("{n}: {e}")))
            .collect();
        assert!(
            failures.is_empty(),
            "pipeline dry-run failures:\n{}",
            failures.join("\n")
        );
    }

    /// Human-readable multi-line summary (used when tests fail or verbose mode ends).
    pub fn summary(&self) -> String {
        self.stages
            .iter()
            .map(|(name, r)| match r {
                Ok(()) => format!("OK  {name}"),
                Err(e) => format!("ERR {name}: {e}"),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Append a stage outcome to the report (Command pattern: each stage records itself).
pub(crate) fn record(stage: &str, res: Result<(), String>, report: &mut PipelineReport) {
    report.stages.push((stage.to_string(), res));
}

/// Convenience for a successful stage outcome.
pub(crate) fn ok() -> Result<(), String> {
    Ok(())
}

/// Convenience for a failed stage outcome with a static message.
pub(crate) fn err_msg(msg: &str) -> Result<(), String> {
    Err(msg.to_string())
}
