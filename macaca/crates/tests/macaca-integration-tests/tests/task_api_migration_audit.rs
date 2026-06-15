use std::fs;
use std::path::{Path, PathBuf};

struct AuditTarget {
    relative_path: &'static str,
}

struct ForbiddenPattern {
    pattern: &'static str,
    replacement: &'static str,
}

const TARGETS: &[AuditTarget] = &[
    AuditTarget {
        relative_path: "crates/services/macaca-tools/src/todo/worker_board.rs",
    },
    AuditTarget {
        relative_path: "crates/services/macaca-tools/src/todo/create_todo.rs",
    },
    AuditTarget {
        relative_path: "crates/services/macaca-tools/src/todo/create_todos.rs",
    },
    AuditTarget {
        relative_path: "crates/services/macaca-tools/src/todo/plan_space.rs",
    },
    AuditTarget {
        relative_path: "crates/shells/macaca-web/src/framework_toolkit/builder.rs",
    },
    AuditTarget {
        relative_path: "crates/shells/macaca-web/src/framework_toolkit/agent_tools.rs",
    },
    AuditTarget {
        relative_path: "crates/shells/macaca-web/src/loop_manager/plan_loop_orchestrator.rs",
    },
    AuditTarget {
        relative_path: "crates/shells/macaca-web/src/loop_manager/worker_loop_orchestrator.rs",
    },
    AuditTarget {
        relative_path: "crates/shells/macaca-web/src/routes/todos.rs",
    },
    AuditTarget {
        relative_path: "crates/tests/macaca-integration-tests/src/pipeline_dry_run.rs",
    },
];

const FORBIDDEN_PATTERNS: &[ForbiddenPattern] = &[
    ForbiddenPattern {
        pattern: "TaskBoard::new(",
        replacement: "TaskBoard::for_agent(",
    },
    ForbiddenPattern {
        pattern: "TaskSpace::new(",
        replacement: "TaskSpace::for_session(",
    },
    ForbiddenPattern {
        pattern: ".claim_next(",
        replacement: ".claim_next_task(",
    },
    ForbiddenPattern {
        pattern: ".start_task(",
        replacement: ".mark_task_in_progress(",
    },
    ForbiddenPattern {
        pattern: ".submit_for_review(",
        replacement: ".submit_task_for_review(",
    },
    ForbiddenPattern {
        pattern: ".review_task(",
        replacement: ".apply_review_result(",
    },
    ForbiddenPattern {
        pattern: ".skip_task(",
        replacement: ".cancel_task(",
    },
    ForbiddenPattern {
        pattern: ".create_and_assign(",
        replacement: ".create_task_assignment(",
    },
    ForbiddenPattern {
        pattern: "PlanLoop::new(",
        replacement: "PlanLoop::with_components(",
    },
    ForbiddenPattern {
        pattern: "WorkerLoop::new(",
        replacement: "WorkerLoop::with_components(",
    },
    ForbiddenPattern {
        pattern: ".run(shutdown",
        replacement: ".run_with_default_template(shutdown",
    },
    ForbiddenPattern {
        pattern: ".run(shutdown_clone",
        replacement: ".run_with_default_template(shutdown_clone",
    },
];

fn macaca_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from CARGO_MANIFEST_DIR")
}

fn strip_strings_and_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut in_char = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut escape = false;

    while let Some(ch) = chars.next() {
        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                out.push('\n');
            } else {
                out.push(' ');
            }
            continue;
        }

        if in_block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                let _ = chars.next();
                out.push(' ');
                out.push(' ');
                in_block_comment = false;
            } else if ch == '\n' {
                out.push('\n');
            } else {
                out.push(' ');
            }
            continue;
        }

        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }

            if ch == '\n' {
                out.push('\n');
            } else {
                out.push(' ');
            }
            continue;
        }

        if in_char {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '\'' {
                in_char = false;
            }

            if ch == '\n' {
                out.push('\n');
            } else {
                out.push(' ');
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'/') {
            let _ = chars.next();
            out.push(' ');
            out.push(' ');
            in_line_comment = true;
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            let _ = chars.next();
            out.push(' ');
            out.push(' ');
            in_block_comment = true;
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(' ');
            continue;
        }

        if ch == '\'' {
            in_char = true;
            out.push(' ');
            continue;
        }

        out.push(ch);
    }

    out
}

#[test]
fn upper_layer_task_consumers_do_not_call_retired_task_apis() {
    let root = macaca_root();
    let mut failures = Vec::new();

    for target in TARGETS {
        let path = root.join(target.relative_path);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let sanitized = strip_strings_and_comments(&content);

        for forbidden in FORBIDDEN_PATTERNS {
            if sanitized.contains(forbidden.pattern) {
                failures.push(format!(
                    "{} contains retired task API pattern `{}`; use `{}` instead",
                    target.relative_path, forbidden.pattern, forbidden.replacement
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "retired macaca-task API usage detected in upper-layer consumers:\n{}",
        failures.join("\n")
    );
}
