# arazzo-store

Postgres persistence for Arazzo workflow runs, steps, attempts, and events.

## Setup

```bash
# Start Postgres
docker compose up -d

# Run migrations
arazzo migrate
```

Environment: `DATABASE_URL` or `ARAZZO_DATABASE_URL`

## Schema

- `workflow_docs` — Stored Arazzo documents
- `workflow_runs` — Execution runs with status/inputs/outputs
- `run_steps` — Per-step state with `deps_remaining` counter
- `step_attempts` — Request/response for each attempt
- `run_events` — Append-only event log
