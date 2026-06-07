//! Legacy in-process TaskScheduler HTTP routes.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use macaca_proto::ApplicationId;

use crate::state::AppState;

use super::shared::{err, ErrorResponse};

// ---------------------------------------------------------------------------
// Schedule routes
// GET    /api/apps/{app_id}/schedules           → list_schedules
// POST   /api/apps/{app_id}/schedules           → create_schedule
// GET    /api/apps/{app_id}/schedules/{id}      → get_schedule
// DELETE /api/apps/{app_id}/schedules/{id}      → delete_schedule
// PUT    /api/apps/{app_id}/schedules/{id}/toggle → toggle_schedule
// ---------------------------------------------------------------------------

/// GET /api/apps/{app_id}/schedules — list all schedules for an application
pub async fn list_schedules(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let scheduler = macaca_sdk::task::TaskScheduler::new(
        Arc::clone(&state.persist.session_store),
        macaca_sdk::task::SchedulerConfig::default(),
    );
    let entries = scheduler.list(&app_id).await;
    Ok(Json(
        serde_json::json!({ "schedules": entries, "count": entries.len() }),
    ))
}

/// POST /api/apps/{app_id}/schedules — create a new schedule
///
/// Body (JSON):
/// ```json
/// {
///   "name": "daily-report",
///   "cron_expr": "0 9 * * *",          // optional; use this OR interval_secs
///   "interval_secs": 3600,              // optional; use this OR cron_expr
///   "action": {
///     "kind": "create_goal",
///     "description": "Generate daily report"
///   }
/// }
/// ```
pub async fn create_schedule(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );

    let name = body["name"]
        .as_str()
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "Missing 'name' field".into()))?
        .to_owned();

    let action: macaca_sdk::task::ScheduleAction = serde_json::from_value(body["action"].clone())
        .map_err(|e| err(StatusCode::BAD_REQUEST, format!("Invalid 'action': {e}")))?;

    let entry = if let Some(expr) = body["cron_expr"].as_str() {
        macaca_sdk::task::ScheduleEntry::new_cron(app_id.clone(), name, expr, action)
    } else if let Some(secs) = body["interval_secs"].as_u64() {
        macaca_sdk::task::ScheduleEntry::new_interval(app_id.clone(), name, secs, action)
    } else {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "One of 'cron_expr' or 'interval_secs' is required".into(),
        ));
    };

    let scheduler = macaca_sdk::task::TaskScheduler::new(
        Arc::clone(&state.persist.session_store),
        macaca_sdk::task::SchedulerConfig::default(),
    );
    let created = scheduler.create(entry).await;

    // Lazily start Scheduler loop for this app if not already running
    {
        let handles = state.loops.scheduler_handles.read().await;
        if !handles.contains_key(&app_id) {
            drop(handles);
            let mut handles = state.loops.scheduler_handles.write().await;
            if !handles.contains_key(&app_id) {
                let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
                handles.insert(app_id.clone(), Arc::clone(&shutdown));

                let sched_runner = macaca_sdk::task::TaskScheduler::new(
                    Arc::clone(&state.persist.session_store),
                    macaca_sdk::task::SchedulerConfig::default(),
                );
                let (sched_event_tx, mut sched_event_rx) =
                    tokio::sync::mpsc::channel::<macaca_sdk::task::ScheduleEvent>(64);
                let app_id_sched = app_id.clone();

                tokio::spawn(async move {
                    sched_runner
                        .run(app_id_sched, shutdown, sched_event_tx)
                        .await;
                });

                let state_for_sched = Arc::clone(&state);
                let app_id_for_sched = app_id.clone();
                tokio::spawn(async move {
                    while let Some(event) = sched_event_rx.recv().await {
                        match event {
                            macaca_sdk::task::ScheduleEvent::Triggered { action, .. } => match action {
                                macaca_sdk::task::ScheduleAction::CreateGoal { description } => {
                                    let space = macaca_sdk::task::TaskSpace::for_session(
                                        app_id_for_sched.clone(),
                                        None,
                                        Arc::clone(&state_for_sched.persist.todo_store),
                                    );
                                    space.push_goal(description).await;
                                }
                                macaca_sdk::task::ScheduleAction::CreateTask {
                                    agent,
                                    title,
                                    description,
                                    priority,
                                } => {
                                    let space = macaca_sdk::task::TaskSpace::for_session(
                                        app_id_for_sched.clone(),
                                        None,
                                        Arc::clone(&state_for_sched.persist.todo_store),
                                    );
                                    space
                                        .create_task_assignment(
                                            &agent,
                                            "scheduler",
                                            &title,
                                            &description,
                                            vec![],
                                            priority,
                                            vec![],
                                            None,
                                        )
                                        .await;
                                }
                            },
                        }
                    }
                });

                tracing::info!(app_id = %app_id, "Scheduler started for app");
            }
        }
    }

    Ok(Json(serde_json::json!({
        "schedule_id": created.id.to_string(),
        "name": created.name,
        "next_run_at": created.next_run_at,
        "enabled": created.enabled,
    })))
}

/// GET /api/apps/{app_id}/schedules/{id} — get a single schedule
pub async fn get_schedule(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((app_id, schedule_id)): axum::extract::Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let id = macaca_proto::TaskId(
        uuid::Uuid::parse_str(&schedule_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid schedule_id".into()))?,
    );

    let scheduler = macaca_sdk::task::TaskScheduler::new(
        Arc::clone(&state.persist.session_store),
        macaca_sdk::task::SchedulerConfig::default(),
    );
    let entry = scheduler
        .get(&app_id, &id)
        .await
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "Schedule not found".into()))?;
    Ok(Json(serde_json::to_value(&entry).map_err(|e| {
        err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?))
}

/// DELETE /api/apps/{app_id}/schedules/{id} — delete a schedule
pub async fn delete_schedule(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((app_id, schedule_id)): axum::extract::Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let id = macaca_proto::TaskId(
        uuid::Uuid::parse_str(&schedule_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid schedule_id".into()))?,
    );

    let scheduler = macaca_sdk::task::TaskScheduler::new(
        Arc::clone(&state.persist.session_store),
        macaca_sdk::task::SchedulerConfig::default(),
    );
    scheduler.delete(&app_id, &id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/apps/{app_id}/schedules/{id}/toggle — enable or disable a schedule
///
/// Body: `{ "enabled": true }` or `{ "enabled": false }`
pub async fn toggle_schedule(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((app_id, schedule_id)): axum::extract::Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let id = macaca_proto::TaskId(
        uuid::Uuid::parse_str(&schedule_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid schedule_id".into()))?,
    );
    let enabled = body["enabled"].as_bool().ok_or_else(|| {
        err(
            StatusCode::BAD_REQUEST,
            "Missing boolean 'enabled' field".into(),
        )
    })?;

    let scheduler = macaca_sdk::task::TaskScheduler::new(
        Arc::clone(&state.persist.session_store),
        macaca_sdk::task::SchedulerConfig::default(),
    );
    let ok = scheduler.set_enabled(&app_id, &id, enabled).await;
    if ok {
        Ok(Json(
            serde_json::json!({ "schedule_id": schedule_id, "enabled": enabled }),
        ))
    } else {
        Err(err(StatusCode::NOT_FOUND, "Schedule not found".into()))
    }
}
