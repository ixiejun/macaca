//! Shared helpers for alias resolution policy contract tests.

use macaca_kernel::SystemService;
use macaca_proto::TraceContext;
use macaca_skill::{
    SkillAliasRecord, SkillAliasResolveCommand, SkillAliasResolveResult, SkillAliasUpsertCommand,
    SkillServiceScope, SKILL_ALIAS_RESOLVE_COMMAND, SKILL_ALIAS_UPSERT_COMMAND,
};

use crate::SkillSystemServiceProvider;

use super::fixtures::traced_command;

pub(super) async fn upsert_alias_for_policy_test(
    provider: &SkillSystemServiceProvider,
    trace: &TraceContext,
    record: SkillAliasRecord,
) {
    let command = SkillAliasUpsertCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        record,
    };
    provider
        .call(traced_command(
            SKILL_ALIAS_UPSERT_COMMAND,
            command,
            trace.clone(),
        ))
        .await
        .expect("alias upsert should succeed for policy-status test");
}

pub(super) async fn resolve_alias_for_policy_test(
    provider: &SkillSystemServiceProvider,
    trace: &TraceContext,
    skill_id: &str,
) -> SkillAliasResolveResult {
    let name = skill_id
        .rsplit('/')
        .next()
        .expect("test skill id should contain a final path segment")
        .to_string();
    let command = SkillAliasResolveCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        skill_id: skill_id.into(),
        name: Some(name),
    };
    let result = provider
        .call(traced_command(
            SKILL_ALIAS_RESOLVE_COMMAND,
            command,
            trace.clone(),
        ))
        .await
        .expect("alias resolve should succeed for policy-status test");
    serde_json::from_value(result.output).expect("alias policy-status result should decode")
}
