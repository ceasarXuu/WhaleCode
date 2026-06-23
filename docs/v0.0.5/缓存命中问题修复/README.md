# TaskSpace DeepSeek 缓存命中问题修复立项

- Created: 2026-06-22
- Updated: 2026-06-23
- Version: v0.0.5 cache-hit blocker
- Status: Active, scope corrected
- Owner / Responsible: WhaleCode v0.0.5 runtime
- Related Systems: TaskSpace runtime, DeepSeek official ChatCompletions provider, provider request construction, benchmark cache trace
- Related Links:
  - `docs/v0.0.5/缓存命中问题修复/01-detailed-repair-plan.md`
  - `docs/v0.0.5/缓存命中问题修复/02-implementation-status.md`
  - `coe/2026-06-22-15-24-taskspace-deepseek-cache-hit-rate.md`
  - `scripts/taskspace-benchmark/verify-deepseek-cache-fix.ps1`
  - `scripts/taskspace-benchmark/test-deepseek-cache-verifier.ps1`
- Risk Level: Critical
- Plan Type: Standard
- Recommended AI Agent reasoning level: xhigh

## 1. Scope

本项目只解决一个问题：

```text
TaskSpace 在 DeepSeek 官方 API 上的 input cache 命中率必须稳定达到 95%+。
```

本项目的目标、验收和证据只围绕 DeepSeek input cache 命中率与 provider-visible 请求形态。

## 2. Confirmed Root Cause

DeepSeek 官方 no-tool probe 可以稳定返回高缓存命中，说明 provider 侧缓存机制可用。

TaskSpace 低命中的根因是请求形态：

```text
多轮 TaskSpace provider 请求
+ 动态消息位于前部
+ 大 native tools schema 反复出现在 ChatCompletions 请求热路径
= 大量稳定内容无法作为同一 provider prefix 稳定复用
```

standard 模式和 TaskSpace 的缓存差异来自 provider-visible 请求形态：TaskSpace 更容易在多轮请求中反复携带大 tools schema 与动态状态，破坏 DeepSeek 可复用前缀。

## 3. Fix Direction

采用 `cache_optimized_action_contract`：

- DeepSeek TaskSpace 热路径不再发送 provider-native tools schema；
- provider-visible 前缀固定为稳定 action contract / cache anchor；
- 动态 TaskSpace 状态放在 bounded suffix；
- 本地 runtime 解析模型输出的 action envelope 并映射到已有执行路径；
- 缓存行为通过 provider cache trace 持续观测。

## 4. Acceptance Criteria

缓存项目只按以下指标验收：

| Gate | Passing Standard | Source |
|---|---:|---|
| DeepSeek official usage 字段可见 | `prompt_cache_hit_tokens` / `prompt_cache_miss_tokens` 可读取 | official API response |
| TaskSpace request 2+ cache hit rate | `>= 0.95` | `provider-cache-trace-summary.json` |
| Cache trace coverage | `>= 0.99` | `provider-cache-trace-summary.json` |
| Missing cache usage count | `0` | `provider-cache-trace-summary.json` |
| Native tools schema hot path count | `0` | `provider-cache-trace-summary.json` |
| Tool-free action contract count | `> 0` | `provider-cache-trace-summary.json` |

首个冷启动请求可以单独记录，但不纳入 request 2+ 稳态命中率判断。

## 5. Current Evidence

已记录的 live evidence：

| Metric | Value |
|---|---:|
| official identical second request hit rate | `0.998267` |
| official prefix-extension third request hit rate | `0.996663` |
| TaskSpace request 2+ hit rate | `0.989246` |
| TaskSpace overall hit rate | `0.990786` |
| cache trace coverage | `1` |
| native tools schema hot path count | `0` |
| tool-free action contract count | `10` |

Evidence files:

- `target/deepseek-cache-fix-validation/scope-corrected-cache-only-verify/deepseek-cache-fix-verification.md`
- `target/deepseek-cache-fix-validation/scope-corrected-cache-only-verify/deepseek-cache-fix-verification.json`
- `target/deepseek-cache-fix-validation/benchmark-20260623-115451/single-file-fast-fix/20260623-115451-777`
- `docs/v0.0.5/缓存命中问题修复/03-production-cache-probe-20260623.md`

## 6. Scope Guard

后续执行必须遵守：

1. 只修复或验证与 DeepSeek input cache 命中率直接相关的请求形态、cache trace、usage 字段解析、cache gate。
2. 缓存验收只读取本文列出的 cache summary fields。
3. 缓存验证脚本的 pass/fail 条件必须只来自 DeepSeek cache usage 与 request shape。
4. 文档、脚本和报告必须只声明缓存 gate。
