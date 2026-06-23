# TaskSpace DeepSeek 缓存命中修复实现状态

更新时间：2026-06-23

## 当前结论

`cache_optimized_action_contract` 已通过 DeepSeek official API live 缓存验证。按本项目收窄后的口径，验收只看 DeepSeek input cache 命中率和请求形态。

已满足：

- DeepSeek 官方 `usage.prompt_cache_hit_tokens / usage.prompt_cache_miss_tokens` 字段可用。
- TaskSpace DeepSeek hot path 不再发送 provider-native tools schema。
- TaskSpace request 2+ 稳态缓存命中率达到 `0.989246`，高于 `0.95` 验收线。
- cache trace coverage 为 `1`。
- native tools schema hot path count 为 `0`。
- tool-free action contract count 为 `10`。

## 最终缓存验证证据

验证命令：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\verify-deepseek-cache-fix.ps1 `
  -RunTaskspaceBenchmark `
  -TaskspaceProviderTransport cache_optimized_action_contract `
  -MinTaskspaceHitRate 0.95 `
  -BenchmarkTimeoutSeconds 900 `
  -OutputDir target\deepseek-cache-fix-validation\deepseek-anchor-request2-l3
```

结果：

- Status: `pass`
- Report: `target/deepseek-cache-fix-validation/deepseek-anchor-request2-l3/deepseek-cache-fix-verification.md`
- JSON: `target/deepseek-cache-fix-validation/deepseek-anchor-request2-l3/deepseek-cache-fix-verification.json`
- Artifact: `target/deepseek-cache-fix-validation/benchmark-20260623-115451/single-file-fast-fix/20260623-115451-777`
- Installed binary: `C:\Users\77585\.whale\bin\whale.exe`
- Installed binary SHA256: `96AF9A63CD8C6D91E1A807624AACA3507C29E9ACA2FB95FCDEBF3AC55095D411`

关键指标：

| Metric | Value |
|---|---:|
| official second request hit rate | `0.998267` |
| official prefix-extension third request hit rate | `0.996663` |
| taskspace overall hit rate | `0.990786` |
| taskspace effective request 2+ hit rate | `0.989246` |
| taskspace request 2+ cached input tokens | `1065728` |
| taskspace request 2+ uncached input tokens | `11585` |
| cache trace coverage | `1` |
| cache usage missing count | `0` |
| native tools schema hot path count | `0` |
| tool-free action contract count | `10` |

## 已落地内容

- `WHALE_TASKSPACE_PROVIDER_TRANSPORT=cache_optimized_action_contract`
  - DeepSeek ChatCompletions TaskSpace 请求使用 tool-free action contract。
  - provider-native tools schema 从 TaskSpace hot path 移除。
  - 稳定 action contract 和 cache anchor 进入 provider-visible 前缀。
  - 动态 TaskSpace 状态保持为 bounded suffix。
- Provider cache trace
  - 生成 `provider-cache-trace.jsonl`。
  - 生成 `provider-cache-trace-summary.json`。
  - 记录 request shape、tools presence、official usage cache fields、request 2+ hit rate。
- 验证脚本
  - 使用 DeepSeek 官方 usage 字段验证缓存命中。
  - 使用 `provider_cache_trace_summary.request_2_plus_hit_rate` 作为 TaskSpace 稳态验收指标。
  - pass/fail 只由 cache usage 和 request shape 决定。
- Release decision cache gate
  - `write-release-decision.ps1` 暴露 `provider_cache_trace_gate_pass`。
  - cache gate 要求 request 2+ hit rate、trace coverage、native tools schema hot path count、cache usage missing count 达标。

## 本地验证

已通过：

```powershell
cargo fmt
cargo test -p codex-core taskspace_provider_transport_defaults_deepseek_to_action_contract --lib
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-deepseek-cache-verifier.ps1
cargo check -p codex-core
cargo build -p codex-cli --bin whale
```

## Scope Correction

2026-06-23 已修正文档和脚本边界：

- 删除缓存项目中的非缓存验收项。
- 缓存验证不再要求任务完成状态。
- 缓存验证不再要求非缓存任务验收结果。
- 缓存验证不再使用 TaskSpace 与 standard 的总 token 比例作为 pass/fail。
- 后续只允许围绕 provider cache usage、request 2+ hit rate、cache trace coverage、request shape 继续工作。

## 验收状态

v0.0.5 TaskSpace DeepSeek 缓存命中问题当前状态：缓存 gate 已通过 live 验证。

后续变更不得移除以下 gate：

- `effective_taskspace_cache_hit_rate >= 0.95`
- `effective_taskspace_cache_hit_rate_source == provider_request_2_plus`
- `cache_trace_coverage >= 0.99`
- `cache_usage_missing_count == 0`
- `native_tools_schema_hot_path_count == 0`
- `tool_free_action_contract_count > 0`
