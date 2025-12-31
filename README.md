# arazzo

Rust implementation of the [Arazzo Specification](https://spec.openapis.org/arazzo/latest.html) — parse, validate, plan, and execute API workflows.

```mermaid
graph LR
    CLI[arazzo-cli] --> Core[arazzo-core]
    CLI --> Exec[arazzo-exec]
    CLI --> Store[arazzo-store]
    Exec --> Core
    Exec --> Store
    Store --> Core
```

## Installation

```bash
# Docker (recommended)
docker build -t arazzo .

# From source
cargo build --release
```

## Requirements

- Rust 1.80+
- Postgres 12+

## Quick Start

```bash
# Start Postgres
docker compose up -d

# Run migrations
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/arazzo"
arazzo migrate

# Validate
arazzo validate workflow.yaml

# Execute
arazzo execute workflow.yaml \
  --inputs inputs.yaml \
  --allow-host api.example.com

# Monitor
arazzo status <run-id>
arazzo events <run-id> --follow
```

## Crates

| Crate | Purpose |
|-------|---------|
| `arazzo-core` | Spec models, parser, validator, planner (pure, no async) |
| `arazzo-exec` | Runtime engine (HTTP, OpenAPI, retries, secrets, policy) |
| `arazzo-store` | Postgres persistence for runs/steps/events |
| `arazzo-cli` | CLI interface |

See [arazzo-cli/README.md](arazzo-cli/README.md) for complete command reference.

## Configuration

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | Postgres connection string |
| `ARAZZO_SECRET_*` | Environment-based secrets |

Common flags: `--allow-host`, `--max-concurrency`, `--timeout`, `--events`, `--format`

## License

Apache License 2.0. See [LICENSE](LICENSE).
