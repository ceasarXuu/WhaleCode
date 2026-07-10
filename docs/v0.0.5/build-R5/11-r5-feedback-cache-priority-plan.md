# R5 工具反馈完整性与缓存前缀优先修复计划

> 本计划是 R5 主计划的优先级插入项。它不扩大 runtime 语义权力：工具层只暴露机械执行事实，
> projection/history 只忠实组织上下文；任务成功判断、错误理解和下一步动作仍完全由 Agent 决定。

## 1. 元数据

```text
Created: 2026-07-10
Updated: 2026-07-10
Version: v0.0.5 build-R5
Status: Approved - next execution gate
Owner / Responsible: WhaleCode core runtime
Related Systems: exec/unified_exec, tool result rendering, NodeEvent, provider history,
  Chat request conversion, benchmark cache telemetry
Related Links:
  docs/v0.0.5/build-R5/00-r5-taskspace-simplification-charter.md
  docs/v0.0.5/build-R5/01-r5-phased-simplification-plan.md
  docs/v0.0.5/build-R5/10-r5-phase-e-runtime-boundary.md
Risk Level: High
Plan Type: Full
```

## 2. 问题定义

### 2.1 工具反馈事实不完整

`subscription-billing-repair` 中 Agent 执行：

```bash
conda install pytest -y 2>&1 | tail -20
```

上游 `conda` 因网络/权限失败，但 shell 按默认管道规则返回最后一个命令 `tail` 的退出码 0。
当前工具只显示单个 `Exit code: 0` 和合并正文，形成“shell 状态为 0、正文明确失败”的反馈冲突。
projection 没有丢失这条结果，但工具反馈没有暴露足够的管道执行事实。

这里禁止两种错误修复：

1. runtime 不得扫描 `error`、`failed` 等正文关键词后重写退出码。
2. runtime 不得据此自动重试、安装依赖、选择解释器或纠正 Agent。

### 2.2 provider 历史破坏缓存前缀

E4 只解决“同一次请求出现多份新旧 projection”，当前 composer 仍会从历史中删除已经发送过的
旧 projection，再把最新 projection 追加到末尾：

```text
request 1: H0 + P1
request 2: H0 + A1/T1 + P2
```

因此 request 2 不包含 request 1 的完整输入/输出前缀。`subscription-billing-repair` 当前 R5
cache hit 为 `69504 / 398853 = 17.4%`，可比 standard 为
`464768 / 484630 = 95.9%`。这不是动态 projection 内容本身的问题，而是历史位置被重排。

## 3. 设计依据

1. [GNU Bash Pipelines](https://www.gnu.org/software/bash/manual/html_node/Pipelines.html)：默认管道状态取最后一个命令；`pipefail` 会改变这一机械规则。
2. [GNU Bash Variables](https://www.gnu.org/software/bash/manual/html_node/Bash-Variables.html)：`PIPESTATUS` 暴露最近前台管道各阶段退出状态，可作为诊断候选，但必须在实际 launcher 中验证可采集性。
3. [POSIX Shell Command Language](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/V3_chap02.html)：pipeline、AND-OR list 和异步命令具有不同退出状态规则，不能把 stderr 文本当成统一状态来源。
4. [DeepSeek Context Caching](https://api-docs.deepseek.com/guides/kv_cache/)：缓存命中要求后续请求完整复用已持久化前缀单元；`A+B` 变成 `A+C` 不能复用 `A+B`。

## 4. 目标与非目标

| Goal | Expected Benefit | Verification |
|---|---|---|
| 暴露无歧义的机械执行事实 | Agent 能区分 shell 总状态、管道阶段状态和传输终止，不再被单一状态字段误导 | focused fixtures 和 provider-visible diff |
| standard/TaskSpace 共用同一反馈契约 | 防止 TaskSpace 再做一层反馈解释 | 两种模式同命令输出结构一致 |
| epoch 内 provider history append-only | 恢复 DeepSeek 公共前缀复用 | 最终 Chat wire message/hash LCP |
| projection 更新改为忠实 delta | 降低输入成本且不压缩、改写语义 | snapshot/delta replay 等价测试 |
| 建立可审计收益门 | 缓存收益不再由累计 token 猜测 | request 级 cache/LCP telemetry 与 paired samples |

非目标：

```text
不从 stdout/stderr 文本推断任务成功或失败。
不让 runtime 决定是否重试、换环境、跳过命令或终止任务。
不默认把所有 stderr 当作失败。
不直接全局开启 pipefail，除非 E5.0 证明不会把 SIGPIPE/有意截断等合法用法误报。
不为缓存把工具结果改写成语义摘要。
不删除或覆盖 Agent 已经看到的自然历史。
不增加旧 projection/history 兼容层或双写路径。
```

## 5. 强制执行顺序

```text
R5-E5.0 反馈契约诊断
  -> R5-E5.1 反馈事实实现与透传
  -> R5-G0 最终 Chat wire/LCP 观测
  -> R5-G1 append-only history 修复
  -> R5-F 死代码和模块拆分
  -> R5-G3 复杂样本与 extractor 完整性
  -> R5-H closeout
```

任何前置 phase 未达到 100% 退出门禁时，后续 phase 不得用于声明该项收益。

## 6. Phase R5-E5：工具反馈事实完整性

### E5.0 诊断和契约冻结

任务：

1. 对账 `exec/unified_exec -> tool output renderer -> NodeEvent -> provider history` 全链路。
2. 确认实际 shell、launcher、PTY/non-PTY 和命令包装方式。
3. 用以下命令族验证原始进程退出码和可机械采集的最后前台管道状态：

```bash
exit 7
false | tail -n 1
printf 'warning\n' >&2; exit 0
yes | head -n 1
false && echo skipped
true || echo skipped
```

4. 冻结最小 `ExecOutcomeV2`，字段只允许表达机械事实：

```text
transport_status: completed | timed_out | cancelled | spawn_failed
shell_exit_code: integer | unavailable
pipeline_stage_exit_codes: integer[] | unavailable
termination_signal: integer | unavailable
stdout/stderr/combined_output availability and ref metadata
truncation/ref metadata
```

5. 明确 `pipeline_stage_exit_codes` 的可观测边界；无法可靠采集时必须写 `unavailable`，禁止伪造。

退出门禁：

```text
每个字段有唯一机械来源和 unavailable 规则。
能解释 pytest 样本为何 shell_exit_code=0、上游 stage 非零。
能区分 stderr warning + exit 0 与真实非零终止。
能解释 SIGPIPE/有意 head 截断，不把任一 stage 非零直接提升为任务失败。
诊断阶段不改变生产执行语义。
```

### E5.1 实现和透传

任务：

1. 在 standard 与 TaskSpace 共用的 exec substrate 产生 `ExecOutcomeV2`。
2. tool result、NodeEvent、output ref 和 provider-visible history 原样携带同一事实字段。
3. 人类可读标题改为明确的 `Shell exit code`，避免把它表述成任务级成功结论。
4. 保留原始输出或透明 ref；projection 不重写状态、不注入解释。
5. 增加稳定日志 `tool.exec_outcome_recorded`，记录字段可用性和关联 ID，不记录敏感正文。

退出门禁：

```text
standard/TaskSpace 对同一命令得到结构一致的反馈。
false | tail 的上游非零事实可见，正文不参与状态推断。
warning to stderr + exit 0 保持 shell_exit_code=0。
timed_out/cancelled/spawn_failed 不再折叠成普通 exit code。
普通工具反馈在 projection replacement、裁剪和 ref 路径后仍可恢复。
runtime 没有新增 retry/recovery/next-action 分支。
```

样本：

| Sample | standard | R4 | R5 E5 | 主要观察 |
|---|---|---|---|---|
| `subscription-billing-repair` | 1 次 | 历史基线或 1 次 | 1 次 | pytest 环境探测反馈、重复工具路径 |
| `large-output-ref-smoke` | 1 次 | 历史基线或 1 次 | 1 次 | 大输出/ref 后事实字段完整性 |

## 7. Phase R5-G0：最终 wire 缓存证据

任务：

1. 在 `build_chat_completions_body` 之后记录最终 Chat request 结构。
2. 只记录 request ID、message index、role、content hash/bytes、tools hash、epoch ID 和相邻请求 LCP；禁止记录正文。
3. 同时保留 Responses input hash，明确标记为 pre-wire，不再称为 exact payload。
4. standard/TaskSpace 使用同一采集点和同一计算器。
5. 把 provider 返回的 request 级 cache hit/miss tokens 与 LCP 关联。

退出门禁：

```text
能够定位相邻请求第一个变化的最终 Chat message/JSON path。
能够证明当前 TaskSpace 在旧 projection 位置断前缀，而不是只凭 pre-wire 推断。
standard 与 TaskSpace request telemetry 覆盖率均为 100%。
该 phase 只加诊断，不改变 message 内容和排序。
```

## 8. Phase R5-G1：cache-preserving history

设计：

```text
epoch start: append one faithful map snapshot
ordinary work: append natural assistant/tool messages
map change: append mechanical map/node/event delta
compaction: close current epoch and create one new faithful snapshot
```

任务：

1. 删除每轮 `stale_active_projection_replaced` 的历史中段删除路径。
2. 定义只含 ID、revision、status、binding、edge 和 event/ref 变化的 delta；不生成语义摘要。
3. 用 reducer 证明 `snapshot + ordered deltas` 与当前 map 状态完全一致。
4. compaction 之外禁止删除、替换或重新排序已发送 message。
5. projection uniqueness 从“每请求只有一个快照”改为“一个 epoch snapshot + ordered deltas”，避免旧门禁阻止 append-only。

退出门禁：

```text
无 compaction 时，request N 的完整最终 Chat input/output message 序列是 request N+1 的严格前缀。
delta replay 与 runtime map revision/state 一致，缺 delta 或乱序会 hard fail 测试。
工具 stdout/stderr/exit/ref 原文不因缓存修复被压缩或重写。
controlled 3-run 中，R5 request-2+ cache hit >= 90%，且不低于同轮 standard 超过 5 个百分点。
correctness、Agent completion、map nodes/edges/events 无回退。
```

样本：

| Sample | standard | R4 | R5 G1 | 主要观察 |
|---|---|---|---|---|
| `count-call-stack` | 1 次 | 历史基线或 1 次 | 1 次 | 简单多轮 LCP、缓存、完成语义 |
| `subscription-billing-repair` | 1 次 | 历史基线或 1 次 | 1 次 | 复杂 map delta、工具反馈、缓存成本 |

## 9. 完整性矩阵

| Plan Item | Production Path | Test Evidence | Runtime Evidence | Status |
|---|---|---|---|---|
| E5.0 outcome contract | exec launcher/render/event history | shell matrix fixtures | diagnostic trace | planned |
| E5.1 faithful feedback | standard/TaskSpace shared exec path | focused + ref/projection tests | `tool.exec_outcome_recorded` | planned |
| G0 final wire trace | post Chat-body conversion | hash/LCP fixtures | request-level trace | planned |
| G1 append-only history | provider history/projection/compaction | prefix + reducer + compaction tests | cache hit/miss + LCP | planned |

## 10. 日志矩阵

| Change Link | Success Signal | Failure Signal | Required Fields | Consumer |
|---|---|---|---|---|
| exec outcome capture | `tool.exec_outcome_recorded` | `tool.exec_outcome_incomplete` | call_id, transport_status, shell_exit_code availability, pipeline status availability | Agent/debug |
| feedback projection | `tool.feedback_preserved` | `tool.feedback_fact_dropped` | call_id, event_id, ref_id, fact hash | runtime test/audit |
| final wire trace | `provider.chat_wire_shape_recorded` | `provider.chat_wire_shape_missing` | request_id, epoch_id, message hashes, tools hash | cache audit |
| adjacent request LCP | `provider.chat_wire_prefix_preserved` | `provider.chat_wire_prefix_broken` | previous/current request_id, first_diff_index/path | cache audit |
| map delta replay | `taskspace.map_delta_replay_valid` | `taskspace.map_delta_replay_mismatch` | task_id, map_id, from/to revision, delta_id | runtime/replay |

## 11. 风险与回退

| Risk | Mitigation | Fallback |
|---|---|---|
| 全局 pipefail 把有意 SIGPIPE 当失败 | E5.0 先验证，不默认启用 | 只暴露 stage facts，不改变 shell 状态 |
| 任意 shell 语法无法可靠采集 stage status | 明确 availability 和最后前台管道边界 | 标记 unavailable，保留 shell 原始状态 |
| delta 缺失导致 map 视图不一致 | revision/reducer 强校验和 replay test | 回退 G1 commit，保留 G0 telemetry |
| 缓存目标诱发语义压缩 | forbidden scan + provider-visible diff | 回退消息布局改动，不恢复 snapshot replacement |
| provider best-effort 缓存波动 | controlled 3-run + wire LCP 双证据 | 以 LCP 正确性关 phase，命中率标 residual |

## 12. 完成定义

```text
E5、G0、G1 各自 100% 通过独立退出门禁。
所有代码变更有 focused、冒烟和回归测试。
每个 phase 完成 standard/R4/R5 的 1 到 2 个样本横向记录。
反馈层没有语义推断、自动 retry 或任务级 success 判断。
cache 修复没有摘要、重写或删除自然历史。
文档、日志 schema、代码路径和运行证据一致。
```
