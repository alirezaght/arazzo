-- Required for gen_random_uuid()
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- 1) Versioned Arazzo documents
CREATE TABLE IF NOT EXISTS workflow_docs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  doc_hash text NOT NULL UNIQUE,
  format text NOT NULL CHECK (format IN ('yaml', 'json')),
  raw text NOT NULL,
  doc jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

-- 2) OpenAPI sources tied to a workflow doc (snapshot OR reference)
CREATE TABLE IF NOT EXISTS openapi_sources (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  workflow_doc_id uuid NOT NULL REFERENCES workflow_docs(id) ON DELETE CASCADE,
  source_name text NOT NULL,

  -- Either store the full spec, or store a URL reference
  openapi jsonb,
  url text,
  etag text,
  version text,

  created_at timestamptz NOT NULL DEFAULT now(),

  CONSTRAINT openapi_sources_one_of CHECK (
    (openapi IS NOT NULL) OR (url IS NOT NULL)
  ),
  CONSTRAINT openapi_sources_unique_name UNIQUE (workflow_doc_id, source_name)
);

-- 3) One row per run
CREATE TABLE IF NOT EXISTS workflow_runs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  workflow_doc_id uuid NOT NULL REFERENCES workflow_docs(id) ON DELETE RESTRICT,
  workflow_id text NOT NULL,

  status text NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'canceled')),
  created_by text,
  idempotency_key text,

  inputs jsonb NOT NULL DEFAULT '{}'::jsonb,
  overrides jsonb NOT NULL DEFAULT '{}'::jsonb,
  error jsonb,

  created_at timestamptz NOT NULL DEFAULT now(),
  started_at timestamptz,
  finished_at timestamptz,

  CONSTRAINT workflow_runs_idempotency_unique UNIQUE (created_by, idempotency_key)
);

-- 4) Steps for a run
CREATE TABLE IF NOT EXISTS run_steps (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  run_id uuid NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,

  step_id text NOT NULL,
  step_index int NOT NULL,

  status text NOT NULL CHECK (status IN ('pending', 'running', 'succeeded', 'failed', 'skipped')),

  source_name text,
  operation_id text,

  depends_on text[] NOT NULL DEFAULT '{}'::text[],
  deps_remaining int NOT NULL DEFAULT 0 CHECK (deps_remaining >= 0),

  next_run_at timestamptz,

  outputs jsonb NOT NULL DEFAULT '{}'::jsonb,
  error jsonb,

  started_at timestamptz,
  finished_at timestamptz,

  CONSTRAINT run_steps_unique_step_id UNIQUE (run_id, step_id),
  CONSTRAINT run_steps_unique_step_index UNIQUE (run_id, step_index)
);

-- 5) Dependency edges per run
CREATE TABLE IF NOT EXISTS run_step_edges (
  run_id uuid NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
  from_step_id text NOT NULL,
  to_step_id text NOT NULL,

  PRIMARY KEY (run_id, from_step_id, to_step_id),

  CONSTRAINT run_step_edges_from_fk
    FOREIGN KEY (run_id, from_step_id)
    REFERENCES run_steps (run_id, step_id)
    ON DELETE CASCADE,

  CONSTRAINT run_step_edges_to_fk
    FOREIGN KEY (run_id, to_step_id)
    REFERENCES run_steps (run_id, step_id)
    ON DELETE CASCADE
);

-- 6) Attempts (append-only)
CREATE TABLE IF NOT EXISTS step_attempts (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  run_step_id uuid NOT NULL REFERENCES run_steps(id) ON DELETE CASCADE,

  attempt_no int NOT NULL CHECK (attempt_no >= 1),
  status text NOT NULL CHECK (status IN ('running', 'succeeded', 'failed')),

  request jsonb NOT NULL DEFAULT '{}'::jsonb,
  response jsonb NOT NULL DEFAULT '{}'::jsonb,
  error jsonb,

  duration_ms int,
  started_at timestamptz NOT NULL DEFAULT now(),
  finished_at timestamptz,

  CONSTRAINT step_attempts_unique_attempt UNIQUE (run_step_id, attempt_no)
);

-- 7) Events (append-only timeline)
CREATE TABLE IF NOT EXISTS run_events (
  id bigserial PRIMARY KEY,
  run_id uuid NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
  run_step_id uuid REFERENCES run_steps(id) ON DELETE CASCADE,

  ts timestamptz NOT NULL DEFAULT now(),
  type text NOT NULL,
  payload jsonb NOT NULL DEFAULT '{}'::jsonb
);

-- Indexes that matter
CREATE INDEX IF NOT EXISTS run_steps_claim_idx
  ON run_steps (run_id, status, deps_remaining, next_run_at, step_index);

CREATE INDEX IF NOT EXISTS run_step_edges_from_idx
  ON run_step_edges (run_id, from_step_id);

CREATE INDEX IF NOT EXISTS run_step_edges_to_idx
  ON run_step_edges (run_id, to_step_id);

CREATE INDEX IF NOT EXISTS step_attempts_latest_idx
  ON step_attempts (run_step_id, attempt_no DESC);

CREATE INDEX IF NOT EXISTS run_events_run_ts_idx
  ON run_events (run_id, ts);

CREATE INDEX IF NOT EXISTS run_events_step_ts_idx
  ON run_events (run_step_id, ts);

CREATE INDEX IF NOT EXISTS workflow_runs_status_created_idx
  ON workflow_runs (status, created_at DESC);

CREATE INDEX IF NOT EXISTS workflow_runs_workflow_created_idx
  ON workflow_runs (workflow_id, created_at DESC);


