# arazzo-core

Pure Arazzo 1.0.x parser, validator, and planner. No async runtime dependencies.

## Usage

```rust
use arazzo_core::{parse_document_str, DocumentFormat, Validate, plan_document, PlanOptions};

// Parse
let parsed = parse_document_str(&content, DocumentFormat::Auto)?;

// Validate
parsed.document.validate()?;

// Plan
let outcome = plan_document(&parsed.document, PlanOptions::default())?;
println!("{}", outcome.plan.unwrap().graph.to_dot("my-workflow"));
```

## Modules

- `types` — Arazzo spec types (`ArazzoDocument`, `Workflow`, `Step`, etc.)
- `validate` — Rule-based validation
- `planner` — Dependency graph, topological sort, execution levels
- `expressions` — Runtime expression parser (`$inputs.x`, `$steps.y.outputs.z`)
