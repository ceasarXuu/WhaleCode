# R5-A TaskSpace 当前状态盘点和计划校准

## 1. 元数据

```text
Created: 2026-07-09
Updated: 2026-07-09
Version: v0.0.5 build-R5
Status: Completed for R5-A
Owner / Responsible: WhaleCode core runtime
Scope: TaskSpace state, projection, gates, tool feedback, R4 baseline evidence
Related Plan: docs/v0.0.5/build-R5/01-r5-phased-simplification-plan.md
```

## 2. R5-A 结论

R5-A 已完成静态盘点和 baseline 归档。本阶段没有修改 runtime 生产代码。

核心结论：

1. 当前 TaskSpace 已经不是单纯的 map/state-machine 工具，而是把 problem ledger、cognitive state、fact coverage、rework guidance、next-valid-actions 等语义控制放进 active path。
2. H203/H204 这类问题的风险点不是“状态机不够强”，而是局部上下文被结构化成 canonical truth 后，又被 projection/gate 放大。
3. R5 不做兼容层；应直接建立最小 node event/ref 路径，再把 projection 变薄，并逐步删除或切断 ledger/gate active path。
4. R4 中已经验证有效的能力主要是工具反馈保真和 large output/ref。R5 必须保留这些底线，不能把它们和语义控制一起删掉。

R5-A 退出门禁状态：

| Gate | Status | Evidence |
|---|---|---|
| active 复杂结构已分类 | pass | 本文第 4-6 节 |
| unknown 不阻塞 R5-B | pass | 未发现阻塞直接建立最小 node event/ref 路径的未知项 |
| baseline 覆盖 simple、feedback、path、large-output | pass | 本文第 7 节 |
| 生产行为不变 | pass | 本阶段仅文档变更 |

## 3. 审计方法

静态代码入口：

| Area | Evidence |
|---|---|
| Task/map/node schema | `third_party/codex-cli/codex-rs/core/src/action_map/map.rs:35` |
| cognitive schema | `third_party/codex-cli/codex-rs/core/src/action_map/cognitive.rs:4` |
| problem ledger schema | `third_party/codex-cli/codex-rs/core/src/action_map/ledger.rs:6` |
| start_task 初始化 | `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:5143` |
| `initial_*` normalization | `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs:1659` |
| tool result record | `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:2577` |
| active projection | `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:11308` |
| fact-source coverage projection | `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:11841` |
| recovery/gate message | `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:13012` |

样本证据入口：

| Sample Class | Evidence |
|---|---|
| H203/H204 path case | `target/r4-h203-app-path-prompt-sqlite-right-20260708-r2/.../pair-001/pair-report.md` |
| simple local smoke | `target/paired-bench-selftest/single-file-fast-fix/20260708-193505-530/output-ref-contract/pair-001/pair-report.md` |
| count-call-stack repaired | `docs/v0.0.5/build-R4/05-phase-benefit-evidence.md` |
| large-output/ref repaired | `docs/v0.0.5/build-R4/05-phase-benefit-evidence.md` |
| R4 sample ledger | `docs/v0.0.5/build-R4/r4-sample-evidence-ledger.json` |

## 4. 状态结构分类

| Structure | Active Role Today | R5 Direction | Reason |
|---|---|---|---|
| `TaskState.id/title/objective/status/owner_session_id` | task identity and ownership | keep | map/state-machine 必要字段 |
| `TaskState.active_map_id/map_ids` | task -> maps index | keep | map 管理必要字段 |
| `TaskState.cognitive_state` | success criteria, facts, output contracts, claims | deactivate from active path | 语义账本，不应成为 runtime 控制层 |
| `TaskState.problem_ledger` | objective, facts, decisions, blockers, next best action | deactivate from active path | 当前 projection/gate 使用它表达语义事实和动作方向 |
| `ActionMapInstance.nodes/edges/leases/results` | map graph and execution records | keep/thin | R5 简洁模型的主体 |
| `MapNode.id/title/kind/status/context/deps` | node-local work context | keep | taskspace 的图化组织核心 |
| `SubagentPlan.why_parallelizable/expected_artifact/acceptance_check/...` | subagent semantic planning | thin/deactivate | 这些字段可作为 Agent-authored note，不能控制普通主路径 |
| `NodeResult.body/tool_success/action_class` | tool feedback record | replace by direct NodeEvent | 方向正确，但不应作为兼容后端长期保留 |
| `NodeResultEvidencePackage.claims/validity/adoption` | result semantic trust/adoption | deactivate | runtime 不应维护语义采纳链 |
| `TaskSpaceTraceEvent` | replay/debug trace | replace/collapse into minimal NodeEvent trace | 不为历史 trace 兼容保留旧结构 |
| `initial_success_criteria/output_contracts/fact_sources` | start_task 初始语义结构 | deactivate as canonical truth | 局部任务文本不应被自动提升为强事实/合同 |

关键发现：

1. `TaskState` 直接嵌入 `cognitive_state` 和 `problem_ledger`，说明语义层已经进入核心状态结构。
2. `normalize_start_task_initial_sections` 会把字符串 `initial_fact_sources` 转成 `observed_from_environment`，并默认挂 `user-request` evidence ref。
3. `normalize_output_contract_array` 会把路径状字符串转成 `artifact_ref`，这会把普通任务文本固化成后续 coverage/closeout 的输入。

## 5. Projection 分类

| Projection Section / Helper | Current Behavior | R5 Direction |
|---|---|---|
| `success_criteria` | 从 `problem_ledger.success_criteria` 输出 | remove from active projection; optional note only |
| `blockers` | 从 ledger 输出 runtime/Agent 混合 blocker | convert to node event/note; no semantic gate meaning |
| `decisions` | 输出 runtime 记录的 semantic decisions | remove from active projection |
| `facts` | 输出 `known_facts` | remove from active projection |
| `relevant_results` | 以 evidence validity 过滤 result | replace with node event refs, no validity inference |
| `recent_tool_feedback` | 输出工具结果摘要 | keep/thin; must preserve success/failure, exit, stderr, path, ref |
| `verified_input_evidence` | 按 successful read/search 组织输入证据 | remove; raw events decide visibility |
| `dependency_read_evidence` | 把上游 inspect read 特别注入 implement 节点 | replace with dependency node event refs |
| `critical_artifact_evidence` | 依据失败编辑/验证重工推断关键 artifact | remove from active projection |
| `fact_source_coverage` | 计算 required/observed artifact 和 alias | remove from projection/gates |
| `result_refs_available` | 暴露可读取 result ref | keep/thin |
| budget/omission audit | 透明说明裁剪 | keep if purely mechanical |

Projection 的问题不是“有结构”，而是它把 runtime 推断出的事实、覆盖率、策略动作和工具反馈混到同一个 provider-visible 表面。R5-C 的目标是保留结构化呈现，但内容只能是 map skeleton、current node、node-local events、refs 和机械裁剪说明。

## 6. Gate 分类

允许保留的硬底线：

| Gate Class | Keep Condition |
|---|---|
| task/node binding | ordinary tool 必须归属当前 task/node，除非明确是硬失败 |
| node lifecycle | 非法状态变更、非法 lease、非法 finish/bind/create 拒绝 |
| tool call/result pairing | call_id、action_class、in-flight reservation 不一致时拒绝 |
| permission/sandbox/security | 权限、安全、协议边界不放松 |
| output size/ref | 大输出转 ref，但必须保留 excerpt/ref 和裁剪说明 |

需要删除或降级的语义 gate：

| Gate / Message | Evidence | R5 Direction |
|---|---|---|
| implement-before-test guidance | `blocked_action_recovery_message` 会指示先 apply_patch、再 finish、再 test | remove strategy text; keep only hard reason |
| `TaskSpaceGateRecoveryV1.next_valid_actions` | recovery message 暴露 next valid actions | remove from model-visible path |
| fact-source coverage blocker | coverage helper 计算 required/unread/alias | remove as blocker; raw read/test events suffice |
| validation failed rework routing | runtime 指示必须创建/bind rework node | convert to event/note; Agent 自行决定 |
| forced inspect transition semantic acceptance | 对重复读/search 等行为作语义归类 | keep only hard lifecycle checks |
| unreviewed/adoption blockers | 依赖 result validity/adoption | remove active control meaning |

R5 的判断标准：runtime 可以拒绝违反状态机底线的动作，但不能告诉 Agent “正确的下一步应该是什么”。

## 7. Baseline 样本

| Sample | Class | Current Evidence | R5 Use |
|---|---|---|---|
| `single-file-fast-fix` selftest | simple success | TaskSpace `business_success=true`, `public_validation_exit_code=0`, `tool_call_count=1`; pair report 因 output-ref expectation 被标记 engineering_unclean | 轻量 smoke，只验证普通成功路径不退化 |
| `count-call-stack` | tool feedback repaired | R4-D 后 TaskSpace solved，wall ratio 1.12，tool ratio 0.55；见 R4 phase benefit | R5 必须保留失败工具反馈和 dependency event 可见性，但不能保留策略注入 |
| `large-output-ref-smoke` | large output/ref | R4-E log bloat 从 490,846,386 bytes 降到 360,600 bytes；后续 R4-D solved | R5 必须保留 raw_ref/excerpt 机制 |
| `sqlite-db-truncate` H203/H204 | path/body reversion negative | 本地 right-only TaskSpace wrong，`public_validation_exit_code=1`，`tool_call_count=19`，`wall_time_ms=801871`，`input_tokens=2541780` | 检查 `/app` 这类局部上下文不再被 projection/ledger 固化放大 |

Baseline 解释：

1. `single-file-fast-fix` 本地 artifact 是诊断 smoke，不是 utility aggregate pass，因为 pair report 明确有 output-ref expectation gate failure。
2. H203/H204 本地 artifact 是负向诊断样本，不是 E3 成功样本。
3. `count-call-stack` 和 `large-output-ref-smoke` 使用 R4 文档/CoE 作为历史回归防线；R5-G 再按当前代码重跑 targeted set。

## 8. R5 后续计划调整

R5-A 盘点后，后续 phase 需要按以下顺序调整：

| Adjustment | New Plan |
|---|---|
| R5-B 直接重写最小事件路径 | 新建或收敛为直接 NodeEvent，不做 `NodeResult/TaskSpaceTraceEvent` 兼容 overlay |
| R5-C 先于 ledger 删除 | 先证明 thin projection 能忠实显示 tool event/ref，再降级 ledger |
| R5-D 拆成 D1/D2 | D1 关闭 `initial_*` canonical truth；D2 降级 `problem_ledger/cognitive_state` active path |
| R5-E 拆成 E1/E2 | E1 移除 model-visible strategy/recovery text；E2 建 hard-gate classifier 再删 semantic gate |
| R5-G 不依赖历史路径 | 重跑最小 targeted samples；历史 artifact 只作为背景参考，不作为门禁数据 |

R5-B 的直接目标是建立一个可验证的最小事件表面，并同步切断旧 result/trace active 依赖：

```text
ordinary tool call/result -> current node event
event -> bounded visible excerpt
large output -> raw_ref + excerpt
projection -> map skeleton + current node + recent node events + refs
old ledger/result/trace -> no compatibility adapter; delete or inactive
```

## 9. Phase A 关闭判定

R5-A 可以关闭，进入 R5-B，但带以下执行约束：

1. R5-B 允许删除或切断 `NodeResult`、`TaskSpaceTraceEvent` active 依赖；不得删除当前运行需要的 large-output/ref 能力。
2. 不允许为了 H203/H204 加新的 runtime 语义纠错规则。
3. 所有 R5-B/C 的测试必须同时检查工具失败语义、路径、stderr/exit/ref 可见。
4. 若 Agent 继续低级重复，第一优先级检查 provider-visible event/ref 是否丢失、裁剪过度或被结构化扭曲。
