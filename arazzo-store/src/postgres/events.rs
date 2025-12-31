use sqlx::PgPool;
use uuid::Uuid;

use crate::store::{NewEvent, NewWorkflowDoc, RunEvent, StoreError, WorkflowDoc};

pub async fn append_event(pool: &PgPool, event: NewEvent) -> Result<(), StoreError> {
    sqlx::query(
        r#"INSERT INTO run_events (run_id, run_step_id, type, payload) VALUES ($1, $2, $3, $4)"#,
    )
    .bind(event.run_id)
    .bind(event.run_step_id)
    .bind(event.r#type)
    .bind(event.payload)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_events_after(pool: &PgPool, run_id: Uuid, after_id: i64, limit: i64) -> Result<Vec<RunEvent>, StoreError> {
    let rows = sqlx::query_as::<_, RunEvent>(
        r#"
SELECT id, run_id, run_step_id, ts, type as event_type, payload
FROM run_events WHERE run_id = $1 AND id > $2 ORDER BY id LIMIT $3
        "#,
    )
    .bind(run_id)
    .bind(after_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn upsert_workflow_doc(pool: &PgPool, doc: NewWorkflowDoc) -> Result<WorkflowDoc, StoreError> {
    let rec = sqlx::query_as::<_, WorkflowDoc>(
        r#"
INSERT INTO workflow_docs (doc_hash, format, raw, doc)
VALUES ($1, $2, $3, $4)
ON CONFLICT (doc_hash) DO UPDATE
SET format = EXCLUDED.format, raw = EXCLUDED.raw, doc = EXCLUDED.doc
RETURNING id, doc_hash, format, raw, doc, created_at
        "#,
    )
    .bind(doc.doc_hash)
    .bind(doc.format.as_str())
    .bind(doc.raw)
    .bind(doc.doc)
    .fetch_one(pool)
    .await?;
    Ok(rec)
}

pub async fn get_workflow_doc(pool: &PgPool, id: Uuid) -> Result<Option<WorkflowDoc>, StoreError> {
    let rec = sqlx::query_as::<_, WorkflowDoc>(
        r#"SELECT id, doc_hash, format, raw, doc, created_at FROM workflow_docs WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

