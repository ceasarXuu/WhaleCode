# I07 脱敏回归证据

这些 fixture 只保留请求身份、事件类型、usage、terminal status 和 payload digest 关系，不包含 prompt、命令、Tool
参数或用户内容。

| Fixture | 来源 | 已知旧行为 | 正确行为 |
|---|---|---|---|
| `usage-double-count-rollout.jsonl` | `WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002` 的 TokenCount 形态 | 8 个完成请求被计为 15 | 8 completed、8 usage、7 state snapshots |
| `attempt-boundary-events.jsonl` + `attempt-boundary-wire.jsonl` | `WAR-20260801-222316-R8-I01-W9-MA-1B64DB37` 的 10/11 对账形态 | `provider_dispatch_trace_mismatch` | 10 boundary requests、11 local attempts、1 local-only failed attempt |

fixture 中的 ID 和 digest 已替换为确定性占位值；事件数量、配对关系和失败位置保持与原始证据一致。

## 固定指纹

| Fixture | SHA-256 |
|---|---|
| `attempt-boundary-events.jsonl` | `7d7294e7b58e0842065111f7fd7f0a7510d1b1f97060e860bd0e3a26dfc7d809` |
| `attempt-boundary-wire.jsonl` | `cde1c1591c5bfc38112547b66a3bb1ca09a990c5ce750f70ebeb0f4e3404e16e` |
| `usage-double-count-rollout.jsonl` | `eee6797144db27dd84b74b5ca654d643a99eb3a2787f4885c57547eee454f991` |
