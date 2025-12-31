# arazzo-cli

Command-line interface for Arazzo workflow execution.

## Commands

| Command | Description |
|---------|-------------|
| `validate` | Parse and validate workflow |
| `plan` | Generate execution plan (supports `--format dot`) |
| `workflows` | List workflows in document |
| `inspect` | Show workflow details |
| `openapi` | Validate OpenAPI resolution |
| `execute` | Execute workflow (blocking) |
| `start` | Start workflow (non-blocking) |
| `resume` | Resume paused/failed run |
| `cancel` | Cancel running workflow |
| `status` | Show run status |
| `trace` | Show execution trace |
| `events` | Show event log (`--follow` for streaming) |
| `metrics` | Show execution metrics |
| `migrate` | Run database migrations |
| `doctor` | Check environment |

## Execute Flags

```
--inputs <file>           JSON/YAML inputs file
--set <key>=<value>       Override input (repeatable)
--allow-host <host>       Allow HTTP to host (repeatable)
--openapi <name>=<path>   OpenAPI source (repeatable)
--store <url>             Postgres connection
--max-concurrency <n>     Global concurrency (default: 10)
--timeout <ms>            Request timeout (default: 30000)
--events <mode>           none|stdout|postgres|both
--webhook-url <url>       Webhook for completion
--secrets <provider>      env|file|aws|gcp
--format <fmt>            text|json
```

## Examples

```bash
# Execute with inputs
arazzo execute workflow.yaml \
  --inputs inputs.json \
  --allow-host api.example.com

# Start async and monitor
RUN_ID=$(arazzo start workflow.yaml --format json | jq -r '.run_id')
arazzo events $RUN_ID --follow

# Generate graph
arazzo plan workflow.yaml --format dot | dot -Tpng -o graph.png
```

## Exit Codes

- `0`: Success
- `2`: Validation failed
- `3`: Run failed
- `4`: Runtime error
