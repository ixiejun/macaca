//! Schedule calculation strategies for the local Scheduler provider.
//!
//! The Scheduler service owns generic due-time materialization, not business
//! dispatch.  This module keeps that calculation behind a Strategy boundary so
//! a future provider can replace the parser with a full cron engine or a remote
//! scheduler without changing the service contract.  The built-in strategy is
//! intentionally conservative: it supports one-shot schedules, fixed intervals,
//! and a bounded five-field cron subset with UTC or fixed-offset time zones.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, Timelike, Utc};
use macaca_proto::{SchedulerJobDefinition, SchedulerMissedRunPolicy, SchedulerScheduleSpec};
use tracing::warn;

use super::store::StoredJob;
use super::support::{LOCAL_PROVIDER_ID, MATERIALIZATION_LIMIT};

/// Default schedule calculator used by the in-memory local provider.
///
/// The type is stateless, which makes it cheap to clone or replace.  It follows
/// the Strategy pattern even though the current provider stores only one
/// instance, because cron parsing is a natural future extension point.
pub(super) struct DefaultScheduleCalculator;

impl DefaultScheduleCalculator {
    /// Return due UTC instants for one stored job at `now`.
    ///
    /// Every returned timestamp is a scheduler memento candidate only.  The
    /// caller still owns missed-run policy, run storage, audit ids, and
    /// dispatch gating.  This separation keeps time calculation side-effect
    /// free and easy to test.
    pub(super) fn due_times(&self, job: &StoredJob, now: DateTime<Utc>) -> Vec<DateTime<Utc>> {
        match &job.definition.schedule {
            SchedulerScheduleSpec::At { run_at } => {
                one_shot_due(*run_at, job.last_scheduled_at, now)
            }
            SchedulerScheduleSpec::Every {
                interval_ms,
                anchor,
            } => interval_due_times(
                anchor.unwrap_or(job.created_at),
                *interval_ms,
                job.last_scheduled_at,
                now,
                &job.definition.missed_run_policy,
            ),
            SchedulerScheduleSpec::CronExpression {
                expression,
                timezone,
                start_after,
                end_before,
            } => cron_due_times(
                expression,
                timezone,
                *start_after,
                *end_before,
                job.last_scheduled_at,
                now,
                &job.definition.missed_run_policy,
            ),
        }
    }
}

/// Apply deterministic stagger to due times for fleet-safe distribution.
///
/// Stagger uses only stable job metadata and never random state.  That makes
/// recovery replayable: the same job id and stagger key always produce the same
/// delay, which is important for auditability in unattended operation.
pub(super) fn apply_stagger(
    definition: &SchedulerJobDefinition,
    due: Vec<DateTime<Utc>>,
) -> Vec<DateTime<Utc>> {
    let Some(stagger) = &definition.stagger_policy else {
        return due;
    };
    if stagger.max_delay_ms == 0 {
        return due;
    }
    let job_id = definition
        .job_id
        .as_ref()
        .map(|job_id| job_id.as_str())
        .unwrap_or("unassigned");
    let mut hasher = DefaultHasher::new();
    job_id.hash(&mut hasher);
    stagger.key.hash(&mut hasher);
    let delay_ms = hasher.finish() % stagger.max_delay_ms;
    let delay = Duration::milliseconds(delay_ms.min(i64::MAX as u64) as i64);
    due.into_iter().map(|time| time + delay).collect()
}

fn one_shot_due(
    run_at: DateTime<Utc>,
    last: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Vec<DateTime<Utc>> {
    if last.is_none() && run_at <= now {
        vec![run_at]
    } else {
        Vec::new()
    }
}

fn interval_due_times(
    anchor: DateTime<Utc>,
    interval_ms: u64,
    last: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    missed: &SchedulerMissedRunPolicy,
) -> Vec<DateTime<Utc>> {
    if interval_ms == 0 || anchor > now || interval_ms > i64::MAX as u64 {
        return Vec::new();
    }
    let interval = Duration::milliseconds(interval_ms as i64);
    let mut cursor = last.map(|time| time + interval).unwrap_or(anchor);
    let limit = catch_up_limit(missed);
    let mut due = Vec::new();
    while cursor <= now && due.len() < limit {
        due.push(cursor);
        cursor += interval;
    }
    apply_missed_policy(due, missed)
}

fn cron_due_times(
    expression: &str,
    timezone: &str,
    start_after: Option<DateTime<Utc>>,
    end_before: Option<DateTime<Utc>>,
    last: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    missed: &SchedulerMissedRunPolicy,
) -> Vec<DateTime<Utc>> {
    let Some(schedule) = CronSchedule::parse(expression) else {
        warn!(
            provider_id = LOCAL_PROVIDER_ID,
            expression, "local scheduler rejected unsupported cron expression"
        );
        return Vec::new();
    };
    let Some(offset) = parse_timezone_offset(timezone) else {
        warn!(
            provider_id = LOCAL_PROVIDER_ID,
            timezone, "local scheduler rejected unsupported cron timezone"
        );
        return Vec::new();
    };
    let lower_bound = last
        .or(start_after)
        .map(|time| time + Duration::minutes(1))
        .unwrap_or_else(|| now - Duration::hours(24));
    let mut cursor = truncate_to_minute(lower_bound);
    let mut due = Vec::new();
    let limit = catch_up_limit(missed);
    while cursor <= now && due.len() < limit {
        if end_before.map(|end| cursor >= end).unwrap_or(false) {
            break;
        }
        if cursor > start_after.unwrap_or(DateTime::<Utc>::MIN_UTC)
            && schedule.matches(cursor.with_timezone(&offset))
        {
            due.push(cursor);
        }
        cursor += Duration::minutes(1);
    }
    apply_missed_policy(due, missed)
}

#[derive(Clone, Debug)]
struct CronSchedule {
    minutes: FieldMatcher,
    hours: FieldMatcher,
    days: FieldMatcher,
    months: FieldMatcher,
    weekdays: FieldMatcher,
}

impl CronSchedule {
    fn parse(expression: &str) -> Option<Self> {
        let fields = expression.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 5 {
            return None;
        }
        Some(Self {
            minutes: FieldMatcher::parse(fields[0], 0, 59)?,
            hours: FieldMatcher::parse(fields[1], 0, 23)?,
            days: FieldMatcher::parse(fields[2], 1, 31)?,
            months: FieldMatcher::parse(fields[3], 1, 12)?,
            weekdays: FieldMatcher::parse(fields[4], 0, 7)?,
        })
    }

    fn matches(&self, local: DateTime<FixedOffset>) -> bool {
        let weekday = local.weekday().num_days_from_sunday();
        self.minutes.matches(local.minute())
            && self.hours.matches(local.hour())
            && self.days.matches(local.day())
            && self.months.matches(local.month())
            && (self.weekdays.matches(weekday) || (weekday == 0 && self.weekdays.matches(7)))
            && valid_local_day(local)
    }
}

#[derive(Clone, Debug)]
enum FieldMatcher {
    Any,
    Exact(u32),
    Step(u32),
}

impl FieldMatcher {
    fn parse(field: &str, min: u32, max: u32) -> Option<Self> {
        if field == "*" {
            return Some(Self::Any);
        }
        if let Some(step) = field.strip_prefix("*/") {
            let step = step.parse::<u32>().ok()?;
            if step == 0 {
                return None;
            }
            return Some(Self::Step(step));
        }
        let value = field.parse::<u32>().ok()?;
        if value < min || value > max {
            return None;
        }
        Some(Self::Exact(value))
    }

    fn matches(&self, value: u32) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(expected) => *expected == value,
            Self::Step(step) => value % *step == 0,
        }
    }
}

fn parse_timezone_offset(value: &str) -> Option<FixedOffset> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("utc") || value == "Z" {
        return FixedOffset::east_opt(0);
    }
    let sign = match value.as_bytes().first().copied()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let rest = &value[1..];
    let (hours, minutes) = if let Some((hours, minutes)) = rest.split_once(':') {
        (hours.parse::<i32>().ok()?, minutes.parse::<i32>().ok()?)
    } else if rest.len() == 4 {
        (
            rest[0..2].parse::<i32>().ok()?,
            rest[2..4].parse::<i32>().ok()?,
        )
    } else {
        (rest.parse::<i32>().ok()?, 0)
    };
    if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
        return None;
    }
    FixedOffset::east_opt(sign * (hours * 3600 + minutes * 60))
}

fn valid_local_day(local: DateTime<FixedOffset>) -> bool {
    NaiveDate::from_ymd_opt(local.year(), local.month(), local.day()).is_some()
}

fn apply_missed_policy(
    due: Vec<DateTime<Utc>>,
    missed: &SchedulerMissedRunPolicy,
) -> Vec<DateTime<Utc>> {
    match missed {
        SchedulerMissedRunPolicy::Skip => due.into_iter().last().into_iter().collect(),
        SchedulerMissedRunPolicy::FireOnce => due.into_iter().last().into_iter().collect(),
        SchedulerMissedRunPolicy::CatchUp { .. } => due,
    }
}

fn catch_up_limit(missed: &SchedulerMissedRunPolicy) -> usize {
    match missed {
        SchedulerMissedRunPolicy::CatchUp { max_runs } => {
            (*max_runs as usize).min(MATERIALIZATION_LIMIT).max(1)
        }
        SchedulerMissedRunPolicy::Skip | SchedulerMissedRunPolicy::FireOnce => {
            MATERIALIZATION_LIMIT
        }
    }
}

fn truncate_to_minute(value: DateTime<Utc>) -> DateTime<Utc> {
    value
        .with_second(0)
        .and_then(|value| value.with_nanosecond(0))
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use macaca_proto::SchedulerTargetCommand;

    use super::*;

    #[test]
    fn cron_matches_fixed_offset_timezone() {
        let due = cron_due_times(
            "30 9 * * *",
            "+08:00",
            None,
            None,
            Some(Utc.with_ymd_and_hms(2026, 5, 20, 2, 0, 0).unwrap()),
            Utc.with_ymd_and_hms(2026, 5, 21, 1, 31, 0).unwrap(),
            &SchedulerMissedRunPolicy::CatchUp { max_runs: 4 },
        );
        assert_eq!(
            due,
            vec![Utc.with_ymd_and_hms(2026, 5, 21, 1, 30, 0).unwrap()]
        );
    }

    #[test]
    fn deterministic_stagger_is_stable_for_same_job_and_key() {
        let job_id = macaca_proto::SchedulerJobId::new("job.stagger").unwrap();
        let mut definition = SchedulerJobDefinition::new(
            macaca_proto::AutonomyScope::global(),
            SchedulerScheduleSpec::At {
                run_at: Utc.with_ymd_and_hms(2026, 5, 21, 0, 0, 0).unwrap(),
            },
            SchedulerTargetCommand::Service(macaca_proto::ServiceTargetCommand {
                service_id: macaca_proto::KernelServiceId::new("service.generic"),
                command_name: macaca_proto::ServiceCommandName::new("generic.dispatch"),
                payload_ref: None,
                metadata: Default::default(),
            }),
        )
        .unwrap();
        definition.job_id = Some(job_id);
        definition.stagger_policy = Some(macaca_proto::SchedulerStaggerPolicy {
            key: "tenant-a".into(),
            max_delay_ms: 10_000,
        });
        let due = vec![Utc.with_ymd_and_hms(2026, 5, 21, 0, 0, 0).unwrap()];
        assert_eq!(
            apply_stagger(&definition, due.clone()),
            apply_stagger(&definition, due)
        );
    }
}
