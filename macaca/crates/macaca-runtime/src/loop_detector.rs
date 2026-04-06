//! Loop detection and circuit breaker for the agentic loop.
//!
//! Detects when an agent repeatedly makes identical tool calls and either warns
//! the agent or terminates the loop to prevent infinite loops.

use std::collections::VecDeque;
use sha2::{Digest, Sha256};

/// Configuration for the loop detector.
#[derive(Debug, Clone)]
pub struct LoopDetectorConfig {
    /// How many recent hashes to keep in the sliding window.
    pub window_size: usize,
    /// Consecutive identical hashes before issuing a warning.
    pub repeat_threshold: usize,
    /// Consecutive identical hashes before forcing termination.
    pub max_repeats: usize,
}

impl Default for LoopDetectorConfig {
    fn default() -> Self {
        Self {
            window_size: 10,
            repeat_threshold: 3,
            max_repeats: 5,
        }
    }
}

/// Action to take after recording a tool call.
pub enum LoopDetectorAction {
    /// No issue detected; continue normally.
    Continue,
    /// Inject a warning message to the agent.
    Warn(String),
    /// Force terminate the loop.
    Terminate(String),
}

/// Detects repeated tool calls and triggers warnings or termination.
pub struct LoopDetector {
    config: LoopDetectorConfig,
    /// Sliding window of recent SHA-256 hashes.
    recent_hashes: VecDeque<String>,
    /// Count of consecutive identical hashes.
    consecutive_same: usize,
    /// Hash of the last recorded tool call.
    last_hash: Option<String>,
}

impl LoopDetector {
    pub fn new(config: LoopDetectorConfig) -> Self {
        Self {
            config,
            recent_hashes: VecDeque::new(),
            consecutive_same: 0,
            last_hash: None,
        }
    }

    /// Record a tool call and check for loop conditions.
    ///
    /// `tool_name` and `args` are hashed together to produce a fingerprint.
    /// Returns the action to take based on the current repetition count.
    pub fn record_tool_call(&mut self, tool_name: &str, args: &str) -> LoopDetectorAction {
        let hash = Self::compute_hash(tool_name, args);

        if Some(&hash) == self.last_hash.as_ref() {
            self.consecutive_same += 1;
        } else {
            self.consecutive_same = 1;
        }

        self.last_hash = Some(hash.clone());
        self.recent_hashes.push_back(hash);
        if self.recent_hashes.len() > self.config.window_size {
            self.recent_hashes.pop_front();
        }

        if self.consecutive_same >= self.config.max_repeats {
            LoopDetectorAction::Terminate(format!(
                "Circuit breaker triggered: {} consecutive identical tool calls detected. Terminating to prevent infinite loop.",
                self.consecutive_same
            ))
        } else if self.consecutive_same >= self.config.repeat_threshold {
            LoopDetectorAction::Warn(format!(
                "Warning: You have made {} consecutive identical tool calls ({}). You may be stuck in a loop. Try a different approach.",
                self.consecutive_same, tool_name
            ))
        } else {
            LoopDetectorAction::Continue
        }
    }

    fn compute_hash(tool_name: &str, args: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(tool_name.as_bytes());
        hasher.update(b"|");
        hasher.update(args.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Reset the detector state (e.g. when a new conversation turn starts).
    pub fn reset(&mut self) {
        self.recent_hashes.clear();
        self.consecutive_same = 0;
        self.last_hash = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_detector() -> LoopDetector {
        LoopDetector::new(LoopDetectorConfig {
            window_size: 10,
            repeat_threshold: 3,
            max_repeats: 5,
        })
    }

    #[test]
    fn no_loop_on_varied_calls() {
        let mut d = make_detector();
        assert!(matches!(d.record_tool_call("read", "a"), LoopDetectorAction::Continue));
        assert!(matches!(d.record_tool_call("write", "b"), LoopDetectorAction::Continue));
        assert!(matches!(d.record_tool_call("read", "c"), LoopDetectorAction::Continue));
    }

    #[test]
    fn warn_on_repeat_threshold() {
        let mut d = make_detector();
        d.record_tool_call("read", "x");
        d.record_tool_call("read", "x");
        let action = d.record_tool_call("read", "x");
        assert!(matches!(action, LoopDetectorAction::Warn(_)));
    }

    #[test]
    fn terminate_on_max_repeats() {
        let mut d = make_detector();
        for _ in 0..4 {
            d.record_tool_call("read", "x");
        }
        let action = d.record_tool_call("read", "x");
        assert!(matches!(action, LoopDetectorAction::Terminate(_)));
    }

    #[test]
    fn reset_clears_state() {
        let mut d = make_detector();
        for _ in 0..4 {
            d.record_tool_call("read", "x");
        }
        d.reset();
        // After reset, three more identical calls should only warn, not terminate
        d.record_tool_call("read", "x");
        d.record_tool_call("read", "x");
        let action = d.record_tool_call("read", "x");
        assert!(matches!(action, LoopDetectorAction::Warn(_)));
    }

    #[test]
    fn different_args_break_streak() {
        let mut d = make_detector();
        d.record_tool_call("read", "a");
        d.record_tool_call("read", "a");
        // Different args — streak breaks
        let action = d.record_tool_call("read", "b");
        assert!(matches!(action, LoopDetectorAction::Continue));
    }
}
