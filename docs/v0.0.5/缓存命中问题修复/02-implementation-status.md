# TaskSpace DeepSeek 缓存命中修复实现状态

更新时间：2026-06-23

## 当前结论

`cache_optimized_action_contract` 已通过 DeepSeek official API live 验证，满足本项目的缓存命中验收口径：

- DeepSeek 官方 `usage.prompt_cache_hit_tokens / usage.prompt_cache_miss_tokens` 字段可用。
- TaskSpace DeepSeek hot path 不再发送 provider-native tools schema。
- TaskSpace request 2+ 稳态缓存命中率达到 `0.989246`，高于 `0.95` 验收线。
- live benchmark 运行成功，验证脚本不再允许“缓存达标但任务失败”的假阳性通过。

## 最终验证证据

验证命令：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\verify-deepseek-cache-fix.ps1 `
  -RunTaskspaceBenchmark `
  -TaskspaceProviderTransport cache_optimized_action_contract `
  -MinTaskspaceHitRate 0.95 `
  -MinTaskspaceImprovementRatio 1.0 `
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
| native tools schema hot path count | `0` |
| tool-free action contract count | `10` |
| verification status | `pass` |

## 已落地内容

- `WHALE_TASKSPACE_PROVIDER_TRANSPORT=cache_optimized_action_contract`
  - DeepSeek ChatCompletions TaskSpace 请求使用 tool-free action contract。
  - provider-native tools schema 从 TaskSpace hot path 中移除。
  - 稳定 action contract 和 DeepSeek cache anchor 进入 provider instructions 前缀。
  - 动态 TaskSpace 状态保持为短 active-state suffix。
- `TaskSpaceActionV1`
  - 模型输出 JSON action，本地映射到现有 `shell_command`、`apply_patch`、`taskspace_control` 等执行路径。
  - 保留 node kind 策略校验。
  - 支持 `control_action`、`control_type` 等常见别名归一化。
  - 支持成功验证后的 `final_synthesis` / `finish_node` 直接收敛为 final answer，避免预算末端再消耗一轮模型请求。
- Provider cache trace
  - 生成 `provider-cache-trace.jsonl` 和 `provider-cache-trace-summary.json`。
  - 记录 request shape、tools presence、official usage cache fields、request 2+ hit rate。
- 验证脚本
  - 使用 DeepSeek 官方 usage 字段验证缓存命中。
  - 使用 `provider_cache_trace_summary.request_2_plus_hit_rate` 作为 TaskSpace 稳态验收指标。
  - 同时要求 TaskSpace run success，防止任务失败时仅凭缓存指标通过。
- Release decision gate
  - `Write-TaskspaceCostAggregateArtifacts` 会从 TaskSpace/right artifacts 聚合 root 级 `provider-cache-trace-summary.json` 和 `provider-cache-trace.jsonl`。
  - `write-release-decision.ps1` 要求 `provider-cache-trace-summary.json` 存在。
  - release decision 必须满足 request 2+ hit rate、trace coverage、native tools schema hot path count 三个缓存门槛。
  - `test-release-decision.ps1` 覆盖低命中、有 native tools schema、缺失 cache trace 三个反例。
- 默认 transport
  - DeepSeek ChatCompletions TaskSpace 默认选择 `CacheOptimizedActionContract`。
  - 显式设置 `WHALE_TASKSPACE_PROVIDER_TRANSPORT=native_tools` 时保留旧路径作为调试入口。

## 本地验证

已通过：

- `cargo fmt`
- `cargo test -p codex-core taskspace_control_create --lib`
- `cargo test -p codex-core taskspace_finish_node_detects_control_type_alias --lib`
- `cargo test -p codex-core taskspace_provider_transport_defaults_deepseek_to_action_contract --lib`
- `cargo test -p codex-core taskspace_action_contract_node_policy_matrix_blocks_cross_node_actions --lib`
- `cargo check -p codex-core`
- `cargo build -p codex-cli --bin whale`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1`
- 对 live artifact `target/deepseek-cache-fix-validation/benchmark-20260623-115451/single-file-fast-fix/20260623-115451-777` 重新运行 `Write-TaskspaceCostAggregateArtifacts`，root summary 保持 `request_2_plus_hit_rate=0.989246`、`native_tools_schema_hot_path_count=0`。
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\verify-deepseek-cache-fix.ps1 ...`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\write-release-decision.ps1 -RunDir target\deepseek-cache-fix-validation\benchmark-20260623-115451\single-file-fast-fix\20260623-115451-777`
  - overall decision: `fail`, because this L1 artifact is not a complete v0.0.5/E3 release package.
  - cache gate: `provider_cache_trace_gate_pass=True`, `trace_coverage=1`, `request_2_plus_hit_rate=0.989246`, `native_tools_schema_hot_path_count=0`, `tool_free_action_contract_count=10`.

反向验证：

- 对旧 artifact `target/deepseek-cache-fix-validation/benchmark-20260623-112745/single-file-fast-fix/20260623-112746-534` 重新运行验证脚本。
- 该 artifact 之前因脚本漏洞被误判为 pass；修复后结果为 `Status: fail`，因为 TaskSpace `exec_exit_code=1` 且 `business_success=false`。

## 根因闭环

已确认并修复的根因链：

1. TaskSpace native-tools ChatCompletions hot path 反复发送大 tools schema，且 tools schema 位于动态消息之后，DeepSeek 无法稳定复用期望中的 provider prefix。
2. 仅移除 tools schema 不足以达标，因为 TaskSpace 仍会 replay 增长的动态历史，稳定前缀占比不足。
3. 修复后的 action-contract transport 将稳定协议和 cache anchor 放入稳定 provider instructions，将动态状态压缩为 bounded suffix。
4. DeepSeek 官方 usage 字段显示 request 2+ 稳态命中率恢复到 `0.989246`。

## 验收状态

v0.0.5 缓存命中问题当前状态：已通过 L1 live 验收。

后续变更不得移除以下 gate：

- `effective_taskspace_cache_hit_rate >= 0.95`
- `effective_taskspace_cache_hit_rate_source == provider_request_2_plus`
- `cache_trace_coverage >= 0.99`
- `native_tools_schema_hot_path_count == 0`
- TaskSpace run success 必须为 true
- release decision 中 `provider_cache_trace_gate_pass == true`
