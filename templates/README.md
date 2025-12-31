# Workflow Templates

| Template | Description |
|----------|-------------|
| `http-api-call.yaml` | Basic HTTP API call with retry |
| `parallel-fanout.yaml` | Parallel execution with fan-in |
| `conditional-branch.yaml` | Conditional branching |
| `auth-flow.yaml` | Authentication flow |

```bash
cp templates/http-api-call.yaml my-workflow.yaml
arazzo execute my-workflow.yaml --allow-host api.example.com
```
