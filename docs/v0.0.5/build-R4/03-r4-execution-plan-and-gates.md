# R4 执行计划和门禁

> R4 的执行原则：先统一链路契约，再修 P0/P1 path，最后用真实样本证明收益。
> 不允许用硬预算限制掩盖复杂任务，也不允许只靠 prompt 文案要求模型“注意 tool 错误”。

## 3.1 Phase R4-A：Inventory and Static Chain Model

目标：

1. 完整登记 tools 调用和返回路径。
2. 用静态代码推理列出成功、失败、拒绝、timeout、大输出、projection、nested、多 agent/MCP 分支。
3. 标记每个 path 的 owner 和测试缺口。

实现项：

| Item | Output |
|---|---|
| source audit | `direct/action-contract/CodeMode/multi-agent/MCP/large-output` path table |
| branch reasoning | success/failure/reject/timeout/omit/ref 分支矩阵 |
| risk classification | P0/P1/P2 风险和 phase owner |
| unknown path guard | 新增或规划静态 guard，防止新增 tool path 未登记 |

退出门禁：

```text
所有 tool path 状态属于 canonical / needs-fix / intentionally-excluded / out-of-scope。
P0/P1 path 都有后续 phase owner。
没有 unknown path。
```

## 3.2 Phase R4-B：Field Evidence Mining

目标：

1. 从历史 target artifact 和 CoE 中提取真实 tool 现场。
2. 建立样本账本和 failure taxonomy。
3. 区分工程链路问题、模型解题错误、环境/timeout 问题。

必查样本：

| Sample | Reason |
|---|---|
| `single-file-fast-fix` before/after | positive control + no_patch_after_known_fix |
| `count-call-stack` | internal apply_patch failure visibility |
| `multi-file-order-pipeline` | action-contract/schema/policy loop |
| `large-output-ref-smoke` | output-ref、日志膨胀、timeout |
| historical invalid-history CoE | provider protocol pairing |

退出门禁：

```text
每个样本都有 run_dir、症状、证据文件、根因候选、R4 phase owner。
至少一个样本能证明修复后 positive control 通过。
至少一个样本能证明当前仍有 P0/P1 未修问题。
```

## 3.3 Phase R4-C：Tool Feedback Contract and Instrumentation

目标：

建立统一的 tool feedback contract，使 standard provider-visible feedback、TaskSpace map、
projection/ref、trace event 不再各自手写。

建议 contract 字段：

```text
ToolFeedbackEnvelopeV1:
  envelope_id
  call_id
  tool_name
  tool_source
  taskspace_node_id
  action_contract_action_id
  status
  failure_kind
  exit_code
  stdout_preview
  stdout_ref
  stderr_preview
  stderr_ref
  model_visible_item_hash
  provider_payload_item_hash
  projection_action
  projection_reason
```

实现项：

| Item | Output |
|---|---|
| canonical envelope builder | 从 `ToolOutput::to_response_item` 或等价 standard item 生成 |
| trace event | `TaskSpaceToolFeedbackEnvelopeV1` |
| payload proof | 证明下一轮 provider payload 包含 envelope 的 model-visible 语义或 ref |
| map proof | 证明 TaskSpace node/map 使用同一 envelope |

退出门禁：

```text
direct success、direct error、action-contract rejection 至少各有 fixture。
provider payload proof 能追踪 call_id/envelope_id。
map preview 与 provider-visible preview 来自同一 envelope。
```

## 3.4 Phase R4-D：Action-Contract Internal Tool Parity

目标：

修复 action-contract 内部工具调用和 standard 工具调用之间的反馈不一致。

覆盖范围：

| Tool / Case | Required Behavior |
|---|---|
| `apply_patch` grammar error | 下一轮 payload 有具体语法错误和 expected format |
| `apply_patch` path/context failure | 下一轮 payload 有目标路径、失败原因、可重试建议 |
| shell/test command failure | 区分 execution failure、test assertion failure、timeout |
| parse rejection | 以 structured feedback 告诉模型 JSON contract 问题 |
| node policy rejection | 告诉模型当前 node 允许的 action 和被拒原因 |

真实样本门禁：

```text
rerun count-call-stack:
  outcome_taskspace = solved 或明确非 feedback-loss 根因
  failed apply_patch feedback appears in next provider-visible payload
  changed_paths includes expected source file when solved
```

退出门禁：

```text
internal apply_patch failure 不再只存在于 stderr 或 recovery text。
action-contract rejected output 不再被模型无视到继续提交 final/wrong。
recovery 停止条件基于语义进展，而不是粗暴固定尝试次数。
```

## 3.5 Phase R4-E：Projection, Output Ref, and Performance Safeguards

目标：

保证 projection 和 output-ref 提升性能但不丢 tool 语义，也不制造日志膨胀。

实现项：

| Item | Output |
|---|---|
| pair-safe projection | tool call/result 成组 omit/ref/keep |
| projection reason taxonomy | `keep`, `summary`, `ref`, `omit_pair`, `protected` |
| large-output ref | stdout/stderr 大输出写 ref，payload/map 放摘要和 ref |
| log bloat guard | rollout 重复事件采样或去重，但保留可 replay 信息 |
| loop diagnostics | repeated policy violation / repeated same tool failure 指标 |

真实样本门禁：

```text
rerun large-output-ref-smoke:
  no 900s timeout from feedback/log loop
  rollout size controlled and reasoned
  failed tests still visible to agent through summary/ref
  cache hit does not regress below R3 target
```

退出门禁：

```text
large raw output 不再直接拖垮 payload 或 artifact。
projection decision 全部可审计。
invalid tool-call history fixture 通过。
```

## 3.6 Phase R4-F：Multi-Agent, MCP, and CodeMode Coverage

目标：

消除非 direct tools 的盲区。不是所有工具都必须写进 TaskSpace map，但每个排除都要有理由。

覆盖范围：

| Path | R4 Requirement |
|---|---|
| CodeMode nested tools | 至少 provider-visible feedback 不丢，并有 parent trace/ref |
| multi-agent control tools | control 类可排除 map，但 agent result/error 需要归属 |
| MCP tools | 统一 large-output/ref/projection 和 provider proof |
| tool-search outputs | 确认 recent-output summary 是否需要覆盖 |

退出门禁：

```text
所有 non-direct tool path 有 coverage status。
intentionally-excluded path 有测试证明不会影响 agent-visible feedback。
```

## 3.7 Phase R4-G：Benchmark Gates and Benefit Validation

目标：

用真实样本证明 R4 的 correctness 和性能收益。

最小样本集：

| Sample | Required Measurement |
|---|---|
| `single-file-fast-fix` | solved 保持，wall/tool/token/cache 对比 |
| `count-call-stack` | feedback-loss 消除，TaskSpace 不再 no-patch |
| `multi-file-order-pipeline` | action-contract loop 降低或消除 |
| `large-output-ref-smoke` | timeout/log bloat 消除 |
| invalid-history fixture | provider protocol pass |

报告格式：

```text
sample
outcome_standard
outcome_taskspace
standard_wall_time_ms
taskspace_wall_time_ms
taskspace_wall_time_ratio
standard_tool_calls
taskspace_tool_calls
taskspace_tool_call_ratio
standard_tokens
taskspace_tokens
taskspace_token_ratio
request_2_plus_cache_hit_rate
tool_feedback_loss_count
projection_omit_count_by_reason
rollout_size_bytes
```

收益判断：

| Metric | Pass Signal |
|---|---|
| correctness | known feedback-loss samples 不再 wrong/no_patch |
| semantic fidelity | tool stderr/path/status 在下一轮 payload 或 ref 可见 |
| protocol safety | no invalid tool-call history |
| performance | wall time/log size/request loop 明显下降，不能只靠 hard cap |
| cache | request 2+ hit rate `>= 0.95` |

## 3.8 Phase R4-H：Closeout

完成条件：

1. 代码、测试、样本证据和文档全部指向同一 HEAD。
2. R4 phase 表中没有未解释的未完成项。
3. 每个未完全解决的问题都有明确降级等级、证据和后续 owner。
4. 工程层 closeout 不伪装成 formal E3 pass。
5. 按项目规则提交并 push 所有改动。

closeout 文档必须回答：

```text
R4 实际修复了哪些 tool path？
哪些 path 被证明本来就是安全的？
哪些 path 被 intentionally excluded，为什么？
每个 known-bad sample 的结果如何？
TaskSpace 相对 standard 的时长/token/tool-call 倍数如何？
DeepSeek cache hit 是否保持？
是否可以进入下一轮 E3 或还需要 R5？
```
