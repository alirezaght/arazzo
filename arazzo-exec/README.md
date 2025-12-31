# arazzo-exec

Runtime engine for executing Arazzo workflows.

## Features

- **OpenAPI resolution** — Resolve `operationId`/`operationPath` to HTTP methods/paths
- **Secrets** — `env://`, `file://`, optional `aws-sm://`, `gcp-sm://`
- **Policy** — Allowed hosts, SSRF protection, request/response limits
- **Retry** — Exponential backoff, jitter, `Retry-After` support
- **Events** — Emit execution events to stdout/postgres

## Usage

```rust
let executor = Executor::new(config, store, http_client, secrets, policy, events);
let result = executor.execute_run(run_id, workflow, compiled, inputs).await;
```

## Cargo Features

```toml
[dependencies]
arazzo-exec = { path = "../arazzo-exec", features = ["aws-secrets", "gcp-secrets"] }
```
