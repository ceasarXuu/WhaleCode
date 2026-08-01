# R5 工具反馈完整性与缓存前缀优先修复计划

> 本计划是 R5 主计划的优先级插入项。它不扩大 runtime 语义权力：工具层只暴露机械执行事实，
> projection/history 只忠实组织上下文；任务成功判断、错误理解和下一步动作仍完全由 Agent 决定。

## 1. 元数据

```text
Created: 2026-07-10
Updated: 2026-07-10
Version: v0.0.5 build-R5
Status: Complete - E5/G0/G1 gates passed; proceed to R5-F
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
| map 更新沿用自然工具历史 | 不增加 runtime 语义表面，并让 Agent 可回看自己已执行的状态动作 | 调用/反馈成对保留与严格前缀测试 |
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

实施结果（2026-07-10）：E5.0/E5.1 已完成。共享 `ExecOutcome` 已贯通 tool result、
NodeEvent、history、app-server 和 TUI；不可可靠采集的 pipeline stage 明确为 `unavailable`，
没有启用全局 `pipefail`。`subscription-billing-repair` paired sample 中，R5 对 pytest、pip 和
手工验证的每次反馈均保留机械状态和原始正文，隐藏 oracle 为 0。standard 同样遭遇 pytest
缺失且进行了更多环境搜索，因此该样本不支持“TaskSpace 反馈丢失导致重复排障”的解释。

| E5 实跑 | standard | R4 | R5 E5 |
|---|---:|---:|---:|
| hidden oracle exit | 0 | 历史基线未提供同轮数据 | 0 |
| tool calls | 24 | 历史基线未提供同轮数据 | 32 |
| wall time | 113.653s | 历史基线未提供同轮数据 | 122.011s |
| pytest 环境相关调用 | 10+ | 历史基线未提供同轮数据 | 7 |

公共 validation 两侧均因 harness 的 `/home/zhangxu/miniconda3/bin/python` 缺少 pytest 而返回
1；这与两侧 patch 的 hidden oracle=0 分离记录，不作为 E5 correctness 失败。

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

实施结果（2026-07-10）：诊断代码和真实门禁均已完成。新增
`provider-chat-wire-trace-v1` 从共享 `build_chat_completions_body` 结果记录无正文 message shape、
tools hash、相邻 LCP/首差异和 request usage；standard/TaskSpace 由 benchmark 注入各自 trace path，
不再依赖 action-map event 或 32MB rollout 扫描。

`count-call-stack` 真实 paired 诊断中，standard 6 个请求的 5 次相邻比较全部保持严格前缀，
request-2+ cache hit 为 95.91%。修复前 R5 在人工中止前放大到 114 个请求，113 次相邻比较中
只有 2 次保持前缀，request-2+ cache hit 为 14.64%。最终 wire 的首差异位于 request 1 的 epoch
snapshot；114 个请求中的 tools hash 和首个 system message hash 各自始终唯一，排除了 tool/system
表面抖动。raw rollout 还证明 composer 删除了 Agent 已成功执行的 `taskspace_control(bind_node)`
调用与 `TaskSpace main node bound` 输出，缓存失效和重复绑定来自同一个历史改写机制。

## 8. Phase R5-G1：cache-preserving history

设计：

```text
epoch start: append one faithful map snapshot
ordinary work: append natural assistant/tool messages
map change: preserve the Agent's exact taskspace_control call and tool output
compaction: close current epoch and create one new faithful snapshot
```

任务：

1. 删除每次 sampling 前“删除旧 projection、生成新 projection”的生产路径。
2. 每个 epoch 只在历史缺少快照时写入一次机械 map snapshot；稳态请求不刷新快照。
3. map/node 变化直接复用 Agent 原始 `taskspace_control` 调用参数和工具原始输出作为自然 delta journal；不新增 runtime delta 文本、摘要或 reducer。
4. provider composer 忠实保留 TaskSpace control call/output pair、普通 assistant/tool 历史和状态机错误反馈；不为旧数据隐藏重复 projection，重复 snapshot 由 producer invariant 和 scanner 直接报错。
5. compaction 之外禁止删除、替换或重新排序已经发送的 message；compaction 后由当前 map 建立一个新 epoch snapshot。
6. TaskSpace 始终使用 provider native tools；禁用旧 action-contract transport，物理删除留在 Phase F。

退出门禁：

```text
无 compaction 时，request N 的完整最终 Chat input/output message 序列是 request N+1 的严格前缀。
initialize/create/bind/finish/state_commit 的调用参数和输出按原顺序留在后续请求中。
工具 stdout/stderr/exit/ref 原文不因缓存修复被压缩或重写。
controlled 3-run 中，R5 request-2+ cache hit >= 90%，且不低于同轮 standard 超过 5 个百分点。
correctness、Agent completion、map nodes/edges/events 无回退。
```

样本：

| Sample | standard | R4 | R5 G1 | 主要观察 |
|---|---|---|---|---|
| `count-call-stack` | 1 次 | 历史基线或 1 次 | 1 次 | 简单多轮 LCP、缓存、完成语义 |
| `subscription-billing-repair` | 1 次 | 历史基线或 1 次 | 1 次 | 复杂状态工具 journal、工具反馈、缓存成本 |

实施结果（2026-07-10）：G1 已完成。sampling loop 不再删除旧 projection 并逐请求写入新
projection；稳态历史每个 epoch 只保留一个 snapshot。后续 map 变化直接沿用 Agent 原始
`taskspace_control` call/output，不增加 runtime 生成的 delta、摘要或 reducer。TaskSpace transport
固定为 provider native tools，旧 action-contract 物理删除留在 R5-F。

首次 paired 验证中 standard/R5 均 solved 且各为 13 个请求；双方 12/12 相邻请求均保持严格
前缀，R5 request-2+ cache hit 97.54%，standard 为 97.52%。随后受控 3-repeat 全部双方 solved：

| Pair | standard requests / hit | R5 requests / hit | R5 prefix | R5 state control |
|---|---:|---:|---:|---|
| 1 | 8 / 96.69% | 13 / 97.01% | 12/12 | initialize 1, finish 3, bind 0 |
| 2 | 8 / 96.75% | 21 / 98.03% | 20/20 | initialize 1, finish 3, bind 0 |
| 3 | 6 / 96.27% | 17 / 97.66% | 16/16 | initialize 1, finish 3, bind 0 |

运行目录：`target/r5-g1-repeats/count-call-stack/20260710-210444-351`。R4 仅有历史 solved
基线，缺少同口径 final-wire trace，不能伪造 cache 对照。请求轮数仍高于 standard，但该 residual
已经与反馈丢失/cache break 分离，禁止通过恢复 runtime hard stop 或语义约束处理。

复杂样本采用 right-only 补验：25 个请求的 24/24 相邻前缀保持，request-2+ cache hit 98.14%，
Agent complete，hidden oracle=0，状态工具为 initialize 1、finish 3、合法 bind 1，无重复 bind。
公共 validator 因固定 Miniconda 环境缺 pytest 返回 1，与 E5 已知环境问题一致。原 paired run 的
standard 侧在完成 2 个请求后静止约 3 分钟且无工具子进程，已人工中止，不纳入 utility 对照。

运行操作经验：benchmark harness 的自测入口是 `test-harness.ps1`，不是
`test-benchmark-harness.ps1`；PowerShell harness 不自动加载仓库 `.env.local`，真实运行前需在
父 shell 中 `source .env.local` 并导出变量，禁止把 API key 写到命令行或 artifact。

## 9. 完整性矩阵

| Plan Item | Production Path | Test Evidence | Runtime Evidence | Status |
|---|---|---|---|---|
| E5.0 outcome contract | exec launcher/render/event history | shell matrix fixtures | diagnostic trace | complete |
| E5.1 faithful feedback | standard/TaskSpace shared exec path | focused + ref/projection tests | `tool.exec_outcome_recorded` | complete |
| G0 final wire trace | post Chat-body conversion | hash/LCP fixtures | standard/R5 paired trace | complete |
| G1 append-only history | provider history/projection/compaction | strict-prefix + natural control feedback tests | 3-repeat paired + complex right-only | complete |

## 10. 日志矩阵

| Change Link | Success Signal | Failure Signal | Required Fields | Consumer |
|---|---|---|---|---|
| exec outcome capture | `tool.exec_outcome_recorded` | `tool.exec_outcome_incomplete` | call_id, transport_status, shell_exit_code availability, pipeline status availability | Agent/debug |
| feedback projection | `tool.feedback_preserved` | `tool.feedback_fact_dropped` | call_id, event_id, ref_id, fact hash | runtime test/audit |
| final wire trace | `provider.chat_wire_shape_recorded` | `provider.chat_wire_shape_missing` | request_id, epoch_id, message hashes, tools hash | cache audit |
| adjacent request LCP | `provider.chat_wire_prefix_preserved` | `provider.chat_wire_prefix_broken` | previous/current request_id, first_diff_index/path | cache audit |

## 11. 风险与回退

| Risk | Mitigation | Fallback |
|---|---|---|
| 全局 pipefail 把有意 SIGPIPE 当失败 | E5.0 先验证，不默认启用 | 只暴露 stage facts，不改变 shell 状态 |
| 任意 shell 语法无法可靠采集 stage status | 明确 availability 和最后前台管道边界 | 标记 unavailable，保留 shell 原始状态 |
| 自然状态工具历史缺失导致 Agent 无法回放 | control call/output pair 与 provider strict-prefix 测试 | 回退 G1 commit，保留 G0 telemetry |
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
