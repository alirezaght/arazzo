use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

use crate::store::{AttemptStatus, RunStep, StepAttempt, StoreError};

pub async fn claim_runnable_steps(pool: &PgPool, run_id: Uuid, limit: i64) -> Result<Vec<RunStep>, StoreError> {
    let mut tx = pool.begin().await?;

    let rows = sqlx::query_as::<_, RunStep>(
        r#"
WITH picked AS (
  SELECT id FROM run_steps
  WHERE run_id = $1 AND status = 'pending' AND deps_remaining = 0
    AND (next_run_at IS NULL OR next_run_at <= now())
  ORDER BY step_index
  FOR UPDATE SKIP LOCKED
  LIMIT $2
)
UPDATE run_steps s
SET status = 'running', started_at = COALESCE(started_at, now())
FROM picked WHERE s.id = picked.id
RETURNING s.id, s.run_id, s.step_id, s.step_index, s.status, s.source_name, s.operation_id,
          s.depends_on, s.deps_remaining, s.next_run_at, s.outputs, s.error, s.started_at, s.finished_at
        "#,
    )
    .bind(run_id)
    .bind(limit)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(rows)
}

/// Reset steps that are stuck in 'running' state (e.g., after executor crash).
/// This allows them to be picked up again by claim_runnable_steps.
pub async fn reset_stale_running_steps(pool: &PgPool, run_id: Uuid) -> Result<i64, StoreError> {
    let result = sqlx::query(
        r#"
UPDATE run_steps SET status = 'pending', started_at = NULL
WHERE run_id = $1 AND status = 'running'
        "#,
    )
    .bind(run_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() as i64)
}

pub async fn get_run_steps(pool: &PgPool, run_id: Uuid) -> Result<Vec<RunStep>, StoreError> {
    let rows = sqlx::query_as::<_, RunStep>(
        r#"
SELECT id, run_id, step_id, step_index, status, source_name, operation_id,
       depends_on, deps_remaining, next_run_at, outputs, error, started_at, finished_at
FROM run_steps WHERE run_id = $1 ORDER BY step_index
        "#,
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn mark_step_succeeded(pool: &PgPool, run_id: Uuid, step_id: &str, outputs: JsonValue) -> Result<(), StoreError> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
UPDATE run_steps SET status = 'succeeded', finished_at = now(), outputs = $3, error = NULL
WHERE run_id = $1 AND step_id = $2
        "#,
    )
    .bind(run_id)
    .bind(step_id)
    .bind(outputs)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
UPDATE run_steps d SET deps_remaining = GREATEST(deps_remaining - 1, 0)
FROM run_step_edges e
WHERE e.run_id = $1 AND e.from_step_id = $2 AND e.to_step_id = d.step_id
  AND d.run_id = $1 AND d.status = 'pending'
        "#,
    )
    .bind(run_id)
    .bind(step_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn get_step_outputs(pool: &PgPool, run_id: Uuid, step_id: &str) -> Result<JsonValue, StoreError> {
    let rec: (JsonValue,) = sqlx::query_as(
        r#"SELECT outputs FROM run_steps WHERE run_id = $1 AND step_id = $2 AND status = 'succeeded'"#,
    )
    .bind(run_id)
    .bind(step_id)
    .fetch_one(pool)
    .await?;
    Ok(rec.0)
}

pub async fn schedule_retry(pool: &PgPool, run_id: Uuid, step_id: &str, delay_ms: i64, error: JsonValue) -> Result<(), StoreError> {
    sqlx::query(
        r#"
UPDATE run_steps SET status = 'pending', next_run_at = now() + ($3 * interval '1 millisecond'), error = $4
WHERE run_id = $1 AND step_id = $2
        "#,
    )
    .bind(run_id)
    .bind(step_id)
    .bind(delay_ms)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_step_failed(pool: &PgPool, run_id: Uuid, step_id: &str, error: JsonValue) -> Result<(), StoreError> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
UPDATE run_steps SET status = 'failed', finished_at = now(), error = $3
WHERE run_id = $1 AND step_id = $2
        "#,
    )
    .bind(run_id)
    .bind(step_id)
    .bind(error.clone())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
WITH RECURSIVE to_skip AS (
    SELECT to_step_id AS step_id
    FROM run_step_edges
    WHERE run_id = $1 AND from_step_id = $2
    UNION
    SELECT e.to_step_id
    FROM run_step_edges e
    INNER JOIN to_skip ts ON e.from_step_id = ts.step_id
    WHERE e.run_id = $1
      AND NOT EXISTS (
          SELECT 1 FROM run_steps
          WHERE run_id = $1 AND step_id = e.to_step_id
            AND status IN ('succeeded', 'failed', 'skipped')
      )
)
UPDATE run_steps d
SET status = 'skipped', finished_at = now(), error = $3
FROM to_skip ts
WHERE d.run_id = $1 AND d.step_id = ts.step_id 
  AND d.status = 'pending'
        "#,
    )
    .bind(run_id)
    .bind(step_id)
    .bind(error)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn insert_attempt_auto(pool: &PgPool, run_step_id: Uuid, request: JsonValue) -> Result<(Uuid, i32), StoreError> {
    let rec: (Uuid, i32) = sqlx::query_as(
        r#"
WITH next_no AS (
  SELECT COALESCE(MAX(attempt_no), 0) + 1 AS attempt_no FROM step_attempts WHERE run_step_id = $1
)
INSERT INTO step_attempts (run_step_id, attempt_no, status, request)
SELECT $1, next_no.attempt_no, 'running', $2 FROM next_no
RETURNING id, attempt_no
        "#,
    )
    .bind(run_step_id)
    .bind(request)
    .fetch_one(pool)
    .await?;
    Ok(rec)
}

pub async fn finish_attempt(
    pool: &PgPool,
    attempt_id: Uuid,
    status: AttemptStatus,
    response: JsonValue,
    error: Option<JsonValue>,
    duration_ms: Option<i32>,
    finished_at: Option<DateTime<Utc>>,
) -> Result<(), StoreError> {
    let finished_at = finished_at.unwrap_or_else(Utc::now);
    sqlx::query(
        r#"
UPDATE step_attempts SET status = $2, response = $3, error = $4, duration_ms = $5, finished_at = $6
WHERE id = $1
        "#,
    )
    .bind(attempt_id)
    .bind(status.as_str())
    .bind(response)
    .bind(error)
    .bind(duration_ms)
    .bind(finished_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_step_attempts(pool: &PgPool, run_step_id: Uuid) -> Result<Vec<StepAttempt>, StoreError> {
    let rows = sqlx::query_as::<_, StepAttempt>(
        r#"
SELECT id, run_step_id, attempt_no, status, request, response, error, duration_ms, started_at, finished_at
FROM step_attempts WHERE run_step_id = $1 ORDER BY attempt_no
        "#,
    )
    .bind(run_step_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

