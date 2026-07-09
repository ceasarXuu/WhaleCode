# R5 Phase C 暴露问题后的后续方向校准

> 本文基于 R5-C/C1 的真实失败和修复证据，细化 R5-D/E/G 后续方向。原则不变：
> TaskSpace 是不可绕过的 map/state-machine 工具，不是语义控制器；Agent 负责智能决策；
> runtime 负责硬底线和工具支持；projection 只做忠实上下文再组织和透明裁剪。

Version: v0.0.5 build-R5
Status: follow-up plan
Date: 2026-07-09
Owner: Codex
Related:
  - `docs/v0.0.5/build-R5/06-r5-phase-c1-native-tool-loop-boundary.md`
  - `coe/2026-07-09-21-50-r5-native-tool-loop-agent-no-patch.md`

## 1. Phase C/C1 暴露的问题

| 问题 | 证据 | 判断 |
|---|---|---|
| runtime 执行层去掉 gate 后，projection 仍可能残留旧语义约束 | C1 前 `Current node contract` 显示 inspect node 允许 `read/search/build/test/control`，不含 `edit` | 这是 model-visible 语义污染，不是 Agent 或 native tool-loop 的根因 |
| 旧文案比旧代码更隐蔽 | runtime 不拦 edit，但 Agent 仍被 visible allowed-actions 带向 test/env probe | 后续不能只审查 preflight gate，还必须审查 provider-visible 文案 |
| budget hard stop 会放大上游语义污染 | Agent 已识别 bug 后未 patch，后续环境探测触发 hard stop | request budget 是放大器；不能用新增语义约束修复 |
| native tool-loop 路径需要 ABI 兼容，但不需要语义假设 | `exec_command` / `read_file` alias 归一后可执行 | 工具 alias 是能力层问题，不是策略层问题 |
| 机械空 map 可接受，但必须克制 | C1 允许 runtime 初始化 blank task/map/node/lease | 只允许表达 pending，不允许 seed 任务事实或策略 |

## 2. 后续总原则

1. **先查上下文，再加约束。** Agent 低级失败时，第一优先级检查工具结果、projection、裁剪、引用和失败反馈是否丢失、扭曲、残缺或过度结构化。
2. **model-visible 文案等同运行时行为。** 即使 runtime 不再拒绝，只要 projection/recovery/sentinel 文案暗示下一步策略，仍按越界处理。
3. **硬底线白名单。** Runtime 只允许拒绝机制性错误：无 active map、无 node/lease、协议非法、权限/安全、资源上限、输出过大转 ref。
4. **语义内容只作为 Agent-authored event/note。** `facts`、`decisions`、`output_contracts`、`result_validity` 不再成为 runtime 的 canonical truth。
5. **成本优化不走语义 batch。** request count 高时，优先减少重复 projection、改善 event/ref 透传和裁剪效率；runtime 不替 Agent 合并、排序或跳过动作。

## 3. R5-D 方向调整

R5-D 不只是删除 ledger active path，还要先做 provider-visible 语义残留审计。

| Subphase | Scope | Exit Gate |
|---|---|---|
| R5-D0 | Provider-visible semantic residue inventory | 扫描并分类所有 `state_machine_allowed_actions`、validation/recovery/sentinel/spawn 文案；每条归为 hard baseline、mechanical status、semantic residue、debug-only |
| R5-D1 | `start_task initial_*` 降级 | `initial_*` 不再进入 canonical truth 或 coverage authority；只作为 Agent-authored note/event |
| R5-D2 | `problem_ledger/cognitive_state` active 降级 | active projection/gate 不再依赖 facts/decisions/adoption/result_validity |

D0 判定规则：

```text
保留：
- no active map/node/lease
- missing task path
- invalid node state
- protocol/schema parse failure
- permission/sandbox/security/resource limit
- output ref/crop explanation

删除或降级：
- 告诉 Agent 该 read/search/edit/test/final
- validation_needs_test 这类策略状态
- rejected_by_state_baseline: list_files/search/read_file/apply_patch 这类语义动作禁止
- 根据 validator failure 推断下一步修复策略
- 根据 coverage/fact_source 要求读特定文件
```

## 4. R5-E 方向调整

R5-E 从“gate pruning”细化为“hard baseline classifier + model-visible cleanup”。

| Work Item | Expected Result | Verification |
|---|---|---|
| hard-gate classifier | 每个拒绝都有 `gate_type`：state_machine / protocol / permission / security / resource | 单元测试枚举所有 blocked/recovery message |
| model-visible cleanup | recovery text 只写 hard reason 和机械状态，不写下一步策略 | `rg` 扫描 forbidden phrases；payload fixture diff |
| sentinel cleanup | sentinel 只做 offline observability 或忠实 event，不生成策略性 Agent 指令 | observability artifact 可见；rollout provider text 不含 sentinel guidance |
| action-contract fallback audit | action-contract 仅作为 explicit transport fallback，不成为默认策略层 | DeepSeek default native tools 测试和 config override 测试 |

禁止项：

```text
next_valid_actions
should read / must read / should edit / must edit
validation_needs_test
rejected_by_state_baseline: <semantic tool list>
finish_node_blocker as strategy hint
coverage/fact_source/rework_target as model-visible instruction
```

## 5. R5-G 方向调整

R5-G 需要把 correctness 和成本分开验收。

| Dimension | Gate | Acceptable Result |
|---|---|---|
| correctness | targeted samples standard/R5 对照 | R5 无明确 correctness 回退；失败先归因上下文/反馈 |
| semantic cleanliness | provider-visible scan | 无策略提示、无 action-class contract、无 old recovery hints |
| feedback fidelity | tool result event/ref audit | stdout/stderr/exit/path/ref 可恢复，不主观摘要成策略 |
| request cadence | request/tool/action metrics | 记录真实放大来源；不要求一次性解决，但必须解释 |
| cost regression | token/projection metrics | projection tokens 不因重复旧结构无解释增长 |

样本选择：

```text
count-call-stack:
  覆盖 native tool loop、projection boundary、patch + validation。

sqlite-db-truncate / H203 path case:
  覆盖局部路径文本不被 canonical truth 放大。

large-output-ref-smoke:
  覆盖 ref/crop 忠实传递，不回退到语义摘要。
```

## 6. 当前优先级

1. R5-D0 已完成首轮：provider-visible semantic residue inventory 和明显越界文案清理见 `08-r5-phase-d0-semantic-residue-inventory.md`。
2. R5-D1/D2 已完成：降级 `initial_*`、ledger、cognitive_state active path，见 `09-r5-phase-d-ledger-deactivation.md`。
3. R5-E：为剩余拒绝建立 hard baseline classifier，删除策略性 recovery/sentinel 文案。
4. R5-G：用 targeted samples 区分 correctness、semantic cleanliness、request cadence 三类结果。

## 7. 不做的事

```text
不为 sample pass 增加 runtime 语义约束。
不把 Agent 的错误动作直接归因于模型智能不足。
不把 request count 问题包装成“Agent 必须少读/少测”的提示。
不保留历史兼容分支。
不让 projection 生成任务策略、下一步建议或语义验收结论。
```
