use serde_json::Value as JsonValue;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::store::{NewRun, NewRunStep, NewStep, RunStatus, RunStepEdge, StoreError, WorkflowRun};

pub async fn create_run_with_id(
    pool: &PgPool,
    run_id: Uuid,
    workflow_doc_id: Uuid,
    workflow_id: &str,
    created_by: Option<String>,
    idempotency_key: Option<String>,
    inputs: &JsonValue,
    overrides: &JsonValue,
    steps: &[NewStep],
) -> Result<Uuid, StoreError> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
INSERT INTO workflow_runs
  (id, workflow_doc_id, workflow_id, status, created_by, idempotency_key, inputs, overrides)
VALUES ($1, $2, $3, 'queued', $4, $5, $6, $7)
ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(run_id)
    .bind(workflow_doc_id)
    .bind(workflow_id)
    .bind(&created_by)
    .bind(&idempotency_key)
    .bind(inputs)
    .bind(overrides)
    .execute(&mut *tx)
    .await?;

    insert_steps(&mut tx, run_id, steps).await?;
    insert_edges_from_steps(&mut tx, run_id, steps).await?;

    tx.commit().await?;
    Ok(run_id)
}

pub async fn create_run(
    pool: &PgPool,
    run: NewRun,
    steps: Vec<NewRunStep>,
    edges: Vec<RunStepEdge>,
) -> Result<Uuid, StoreError> {
    let mut tx = pool.begin().await?;

    let run_id = insert_run(&mut tx, run).await?;

    for s in &steps {
        let deps_remaining = s.depends_on.len() as i32;
        sqlx::query(
            r#"
INSERT INTO run_steps
  (run_id, step_id, step_index, status, source_name, operation_id, depends_on, deps_remaining)
VALUES ($1, $2, $3, 'pending', $4, $5, $6, $7)
            "#,
        )
        .bind(run_id)
        .bind(&s.step_id)
        .bind(s.step_index)
        .bind(&s.source_name)
        .bind(&s.operation_id)
        .bind(&s.depends_on)
        .bind(deps_remaining)
        .execute(&mut *tx)
        .await?;
    }

    for e in &edges {
        sqlx::query(
            r#"
INSERT INTO run_step_edges (run_id, from_step_id, to_step_id)
VALUES ($1, $2, $3)
ON CONFLICT DO NOTHING
            "#,
        )
        .bind(run_id)
        .bind(&e.from_step_id)
        .bind(&e.to_step_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(run_id)
}

pub async fn get_run(pool: &PgPool, run_id: Uuid) -> Result<Option<WorkflowRun>, StoreError> {
    let rec = sqlx::query_as::<_, WorkflowRun>(
        r#"
SELECT id, workflow_doc_id, workflow_id, status, created_by, idempotency_key,
       inputs, overrides, error, created_at, started_at, finished_at
FROM workflow_runs WHERE id = $1
        "#,
    )
    .bind(run_id)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn mark_run_finished_enum(
    pool: &PgPool,
    run_id: Uuid,
    status: RunStatus,
    error: Option<JsonValue>,
) -> Result<(), StoreError> {
    sqlx::query(
        r#"
UPDATE workflow_runs SET status = $2, finished_at = now(), error = $3
WHERE id = $1
        "#,
    )
    .bind(run_id)
    .bind(status.as_str())
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_run_started(pool: &PgPool, run_id: Uuid) -> Result<(), StoreError> {
    sqlx::query(
        r#"
UPDATE workflow_runs SET status = 'running', started_at = COALESCE(started_at, now())
WHERE id = $1 AND (status = 'queued' OR status = 'pending')
        "#,
    )
    .bind(run_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_run_finished_str(
    pool: &PgPool,
    run_id: Uuid,
    status: &str,
    error: Option<JsonValue>,
) -> Result<(), StoreError> {
    sqlx::query(
        r#"
UPDATE workflow_runs SET status = $2, finished_at = now(), error = $3
WHERE id = $1
        "#,
    )
    .bind(run_id)
    .bind(status)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn check_run_status(pool: &PgPool, run_id: Uuid) -> Result<String, StoreError> {
    let rec: (String,) = sqlx::query_as(r#"SELECT status FROM workflow_runs WHERE id = $1"#)
        .bind(run_id)
        .fetch_one(pool)
        .await?;
    Ok(rec.0)
}

async fn insert_steps(tx: &mut Transaction<'_, Postgres>, run_id: Uuid, steps: &[NewStep]) -> Result<(), StoreError> {
    for s in steps {
        let deps_remaining = s.depends_on.len() as i32;
        sqlx::query(
            r#"
INSERT INTO run_steps
  (run_id, step_id, step_index, status, source_name, operation_id, depends_on, deps_remaining)
VALUES ($1, $2, $3, 'pending', $4, $5, $6, $7)
ON CONFLICT (run_id, step_id) DO NOTHING
            "#,
        )
        .bind(run_id)
        .bind(&s.step_id)
        .bind(s.step_index)
        .bind(&s.source_name)
        .bind(&s.operation_id)
        .bind(&s.depends_on)
        .bind(deps_remaining)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn insert_edges_from_steps(tx: &mut Transaction<'_, Postgres>, run_id: Uuid, steps: &[NewStep]) -> Result<(), StoreError> {
    for s in steps {
        for dep in &s.depends_on {
            sqlx::query(
                r#"
INSERT INTO run_step_edges (run_id, from_step_id, to_step_id)
VALUES ($1, $2, $3)
ON CONFLICT DO NOTHING
                "#,
            )
            .bind(run_id)
            .bind(dep)
            .bind(&s.step_id)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

async fn insert_run(tx: &mut Transaction<'_, Postgres>, run: NewRun) -> Result<Uuid, StoreError> {
    if run.created_by.is_some() && run.idempotency_key.is_some() {
        let inserted: Option<(Uuid,)> = sqlx::query_as(
            r#"
INSERT INTO workflow_runs
  (workflow_doc_id, workflow_id, status, created_by, idempotency_key, inputs, overrides)
VALUES ($1, $2, 'queued', $3, $4, $5, $6)
ON CONFLICT (created_by, idempotency_key) DO NOTHING
RETURNING id
            "#,
        )
        .bind(run.workflow_doc_id)
        .bind(&run.workflow_id)
        .bind(&run.created_by)
        .bind(&run.idempotency_key)
        .bind(&run.inputs)
        .bind(&run.overrides)
        .fetch_optional(&mut **tx)
        .await?;

        if let Some((id,)) = inserted {
            return Ok(id);
        }

        let existing: (Uuid,) = sqlx::query_as(
            r#"SELECT id FROM workflow_runs WHERE created_by = $1 AND idempotency_key = $2"#,
        )
        .bind(&run.created_by)
        .bind(&run.idempotency_key)
        .fetch_one(&mut **tx)
        .await?;

        return Ok(existing.0);
    }

    let rec: (Uuid,) = sqlx::query_as(
        r#"
INSERT INTO workflow_runs
  (workflow_doc_id, workflow_id, status, created_by, idempotency_key, inputs, overrides)
VALUES ($1, $2, 'queued', $3, $4, $5, $6)
RETURNING id
        "#,
    )
    .bind(run.workflow_doc_id)
    .bind(&run.workflow_id)
    .bind(&run.created_by)
    .bind(&run.idempotency_key)
    .bind(&run.inputs)
    .bind(&run.overrides)
    .fetch_one(&mut **tx)
    .await?;

    Ok(rec.0)
}

