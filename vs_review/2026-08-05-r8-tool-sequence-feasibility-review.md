# Subagent VS Review: R8 Tool 序列容器可行性验证

- Created: 2026-08-05T02:25:44+08:00
- Updated: 2026-08-05T02:37:25+08:00
- Report schema: adversarial-v1
- Task: 独立确认现有证据是否足以支持“Tool 序列容器方案可行，可以进入生产实施”
- Report path: `vs_review/2026-08-05-r8-tool-sequence-feasibility-review.md`
- Review mode: fresh internal subagent
- Source session policy: no inherited main-agent context
- Status: passed with Phase B stop points

## Round 1: 可行性结论证伪

### Review Input

#### Objective

尝试证伪 R8 Tool 序列容器方案的可行性结论。审查只回答方案在现有 Codex/Provider 基建上是否可实现，不把生产代码
尚未接线、旧路径尚未删除或实施后的性能观测缺口自动判为方案不可行。

#### Review Target

- 产品与工程设计中的顶层 Tool 序列容器方案；
- Phase A TS-01～TS-09 的本地合同、Provider 探针和验证结论；
- “没有方案级 Blocking，可以进入生产实施”这一结论的证据充分性。

#### Target Locations

- `docs/v0.0.5/build-R8/tool-sequence-protocol/00-product-definition.md`
- `docs/v0.0.5/build-R8/tool-sequence-protocol/01-execution-ownership-mvp-feasibility-plan.md`
- `docs/v0.0.5/build-R8/tool-sequence-protocol/02-mvt1-native-router-reuse-result.md`
- `docs/v0.0.5/build-R8/tool-sequence-protocol/03-production-engineering-plan.md`
- `docs/v0.0.5/build-R8/tool-sequence-protocol/04-ts04-control-router-seam-result.md`
- `docs/v0.0.5/build-R8/tool-sequence-protocol/05-phase-a-ts01-ts03-validation-result.md`
- `docs/v0.0.5/build-R8/tool-sequence-protocol/06-hosted-container-provider-probe-result.md`
- `docs/v0.0.5/build-R8/tool-sequence-protocol/07-ts06-hosted-response-scope-mvt-result.md`
- `docs/v0.0.5/build-R8/tool-sequence-protocol/08-phase-a-ts05-ts09-complete-validation-result.md`
- `third_party/codex-cli/codex-rs/core/src/tools/taskspace_sequence_schema_contract_tests.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/taskspace_sequence_result_contract_tests.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/taskspace_hosted_binding_contract_tests.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/rooted_dag/sequence_state_contract_tests.rs`
- `scripts/taskspace-benchmark/r8_hosted_container_probe.py`
- `scripts/taskspace-benchmark/test_r8_hosted_container_probe.py`

#### Change Introduction

方案令 TaskSpace Agent 只通过一个顶层序列容器提交 client-managed Tool 和 `taskspace_control`；原生 Tool 的 schema、
Router、handler 和 hook 保持不变。Provider-hosted Tool 仍由 Provider 原生执行，Runtime 收到响应后将真实输出按响应级
节点声明结算。容器只表达 Map 边界和节点归属，不用数组顺序代替业务 DAG。

#### Risk Focus

- 测试是否只验证了测试内自定义数据结构，而未触及决定可行性的真实 Codex/Provider 边界；
- Provider-hosted 输出和容器调用能否在同一响应可靠共存、唯一识别并结算；
- 容器是否真的能无损还原不同种类原生 Tool，而不是只证明 Function/Freeform 两个简单类型；
- `taskspace_control` 与普通 Tool 是否真正复用同一 Router 生命周期，还是隐藏了第二套执行器；
- 七类序列是否遗漏一种会推翻 schema 或执行模型的必要产品行为；
- Tool outcome 与节点 lifecycle 正交是否在现有持久化模型中可实现，而不要求违反根本 DAG invariant；
- 当前证据是否把“尚未实施”错误地提升为不可行，或反过来把“测试 seam 可编译”错误地当成可实施。

#### User-Perspective Review Focus

- Agent 面对容器 schema 时是否仍能理解和使用原生 Tool；
- 合法动作是否能直接生成，而不依赖事后惩罚、语义推断或 reasoning 解析；
- 方案是否保持 TaskSpace 的 Map 工作方式，同时不污染 Standard。

#### Implementation Completeness Focus

本轮不审查 Phase B 是否已经落地。必须检查 Phase A 声称的每一项“可行性证据”是否来自真实边界、生产可复用 seam
或客观 Provider 探针。测试专用 schema、adapter、mock 或 characterization test 不能单独证明生产完成，也不能单独
证明方案不可行。

#### Target Benefit Focus

方案声称的主要收益是原生 Tool 无侵入、共用执行基建、生成前表达合法序列、Hosted 可归属、Standard 隔离。当前不声称
请求数、token、缓存或 Agent 质量收益已经实现；这些效果只能在生产接线后测量。

#### Assumptions To Attack

- Provider 能在同一响应返回 Hosted 结果和容器 Tool call；
- Provider 返回的 Hosted item 有足够稳定的响应内身份；
- Runtime 可把容器 item 还原为现有 typed `ToolCall`；
- 容器 schema 不需要复制或修改每个原生 Tool schema；
- Map 前后边界足以表达必要控制，而 Work 间顺序继续由 DAG 表达；
- Hosted Tool 的执行状态和节点状态可以独立结算；
- Standard 请求不暴露容器即可保持隔离。

#### Adversarial Lenses

- requirements
- state
- input
- failure
- data
- implementation-completeness
- maintenance
- testing
- observability

#### Verification Status

- Rust Core 合同/特征测试 22 项通过；
- `codex-tools` 相关测试 11 项通过；
- Python Hosted 探针单测 2 项通过；
- 既有真实 Provider Hosted+容器探针 2/2；
- 缓存 final-wire 门禁通过；
- 正式生产入口尚未接线，因此 Agent 遵循率、成本和复杂样本表现未测。

#### Reviewer Instructions

- 使用全新内部 subagent session，`fork_context=false`。
- 只读检查，不修改任何文件。
- 直接阅读目标文件和必要生产 seam；不得依赖主 Agent 的结论。
- 尝试构造能推翻方案可行性的具体反例。
- 严格区分：方案级不可行、Phase B 实施任务、实施后效果观测。
- Blocking 只用于会迫使放弃或实质重构容器方案的证据；生产未接线本身不是可行性 Blocking。
- 每项 finding 给出破坏的假设、触发条件、影响、所需证明和精确路径/行号。
- 检查用户理解成本、实施完整性和收益证据，但不要把未声明的性能收益当成失败。

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Local CLI discovery commands: n/a
- Discovered CLI candidates: n/a
- User-recommended alternative agent requested: n/a
- User-recommended agent command: n/a
- User-recommended agent verification: n/a
- User approval requested: n/a
- User-approved CLI command: n/a
- User decision: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 20 minutes | one bounded 10-minute extension | 2 | accepted blocker 修复后必须由新 reviewer 复审 |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| test-validity-adversary | 当前最高风险是测试自证、覆盖空洞或结论超出证据，而非生产实现完整性 | 可行性证据有效性 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| test-validity-adversary | `multi_agent_v1__spawn_agent` | `019fce07-5646-77c1-9ca0-8d7ad3575fff` (Mendel) | spawn tool result in parent session | `fork_context=false` | Round 1 Review Input + target navigation packet | 主会话历史、推理、草稿、结论及完整 diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R1-Mendel | test-validity-adversary | 1 | `019fce07-5646-77c1-9ca0-8d7ad3575fff` | ~8 minutes | completed | 在 20 分钟初始窗口内完成 | completed |

### Reviewer Outputs

#### R1-Mendel

##### Summary

未发现方案级 Blocking，无法证伪“薄容器方向在现有 Codex/DeepSeek Responses 基建上可实现”。现有证据足以支持带停点
进入 Phase B，但不足以声称最终容器合同、全部 typed Tool、恢复路径和持久化正交性已经端到端验证，更不支持直接生产
切换。Reviewer 独立复跑 Hosted binding 5 项、sequence 11 项、state 2 项、Python 探针单测 2 项，全部通过；未执行新的
真实 Provider run。

##### Blocking Findings

- none。最接近 Blocking 的风险是 ToolSearch/typed output 如何通过 Provider 只认识的外层容器 `call_id` 返回；现有证据
  尚未闭合，但存在外层 settlement carrier 和下一请求重派生 schema 的可实施路径，因此属于 Phase B 架构停点。

##### Non-blocking Risks

- **R1：合同测试主要验证测试内候选结构。**
  - Broken assumption: fixture 通过等于生产类型可承载。
  - Failure scenario: 生产 decoder 与测试自定义 envelope、settlement 或 binding ledger 漂移。
  - Trigger condition: TS-11/TS-12 实现真实类型。
  - Impact: “完整生产合同已验证”的表述过强。
  - Proof needed: 测试直接实例化生产 ToolSpec、decoder、settlement 和 Event Store API。
- **R2：MCP、Namespace、ToolSearch、LocalShell 的输入和结果闭环未验证。**
  - Broken assumption: 能进入 Router 等于能无损嵌套并返回。
  - Failure scenario: 内部 ToolSearch 结果没有 Provider 原生内部 `call_id` 对位，或丢失 deferred-tool schema 刷新语义。
  - Trigger condition: 容器内执行 typed Tool。
  - Impact: 需要明确的外层结果 carrier，不能直接把 Function/Freeform adapter 当成全部证明。
  - Proof needed: ToolSearch 到外层结果、下一请求加载新 Tool、成功 dispatch 的完整测试，以及其他 typed Tool 等价闭环。
- **R3：七类正常序列没有表达历史 unbound Hosted 事实的后续补绑定。**
  - Broken assumption: 响应级 `hosted_node_id` 已覆盖完整恢复生命周期。
  - Failure scenario: Hosted 已发生但当轮缺少容器，Agent 后续又不能提交 Provider ID，原始事实永久 unbound。
  - Trigger condition: `tool_choice=auto` 漏容器或容器无效。
  - Impact: 需要 Runtime-issued handle、恢复 item 或新的明确产品限制，但不要求放弃容器。
  - Proof needed: 冻结跨轮恢复合同并验证重启后补绑定。
- **R4：Hosted 唯一身份和持久化绑定只在测试 ledger 中证明。**
  - Broken assumption: 测试 `BTreeMap<provider_id,node>` 等同生产 Event Store。
  - Failure scenario: 生产 rebinding 按 `call_id` 操作，而 Hosted output 使用 item ID；跨响应作用域和重启 replay 不明确。
  - Trigger condition: unbound 恢复、重启或身份冲突。
  - Impact: 绑定无法恢复或产生错误 conflict。
  - Proof needed: 生产 Event Store 上的响应作用域身份、冲突和 replay 测试。
- **R5：状态正交测试同时证明当前 invariant 不兼容目标合同。**
  - Broken assumption: success/failure 释放 reservation 的同效应已足以证明完整正交。
  - Failure scenario: Agent complete/finish 时存在未结算动作，迟到结果不能穿过终态并持久化。
  - Trigger condition: 异步、取消、outcome_unknown 或迟到结果。
  - Impact: 必须重构 DAG invariant/event replay，但不推翻容器。
  - Proof needed: Completed/Blocked/Finished 后迟到结果仍归档且不反写 lifecycle。
- **R6：真实 Provider 探针没有使用最终生成的异构容器 ToolSpec。**
  - Broken assumption: 简单 `node_id/provider_item_id` function 共存可代表最终容器 schema。
  - Failure scenario: 最终 oneOf/namespace/freeform schema 被 Provider 拒绝或 Agent 无法生成。
  - Trigger condition: TS-11 首次生成完整 ToolSpec。
  - Impact: 可能需要调整 schema 表达，但尚无不可行证据。
  - Proof needed: 对实际生成的完整 ToolSpec 做 final-wire、HTTP acceptance 和最小生成验证。

##### User-Perspective Checks

- Usability: unmeasured。当前 fixture 的 `tool: String + input: Value` 没有证明 Agent 能看到并理解最终派生的原 Tool 参数合同。
- Ease of use: unmeasured。漏容器率和复杂序列遵循率只能在生产 ToolSpec 接线后测量。
- Ease of understanding: directional pass。容器、Map 边界和响应级 Hosted 归属的职责可以解释，但历史 unbound 恢复动作尚无
  Agent 可见合同。

##### Implementation Completeness Checks

| Plan Item | Expected Behavior | Production Code Path | Test Evidence | Status | Finding Link |
|---|---|---|---|---|---|
| TS-11 | 最终容器 ToolSpec | 尚未接线 | 测试内 candidate schema | not-started | R1/R6 |
| TS-12 | decoder + persistent Hosted reconciler | 尚未接线 | 测试 ledger | not-started | R3/R4 |
| TS-14 | 全部 client Tool 复用 Router | Function/Freeform seam 已有 | typed Tool 未闭环 | partial | R2 |
| TS-16 | outcome/lifecycle 正交结算 | 当前 invariant 冲突 | characterization test | not-started | R5 |
| Standard isolation | 切换后 final-wire 零差异 | 旧模式分支存在 | 旧基线 | partial | R6 |

##### Target Benefit Checks

| Claimed Benefit | Evidence | Result | Status | Finding Link |
|---|---|---|---|---|
| 原 Router/handler/hook 复用 | Function/Freeform 与 control seam | 方向成立 | weak-evidence | R2 |
| Hosted 同响应共存 | 真实探针 2/2 | 能力成立，不保证容器必达 | weak-evidence | R3/R6 |
| 原 Tool 无侵入 | descriptor 派生方向与现有 seam | typed Tool 待证明 | weak-evidence | R2 |
| 生成前表达合法序列 | candidate schema fixture | 未用最终 Provider schema 测量 | unmeasured | R6 |
| Standard 隔离 | 现有模式分支 | 切换后未验证 | weak-evidence | R6 |
| 成本、质量、遵循率 | 尚无生产容器 | 未测量且未提前宣称实现 | deferred | B1 |

##### Required Fixes

- R1：生产类型落地后让合同测试直接消费生产 ToolSpec、decoder、settlement 和 Event Store。
- R2：把 ToolSearch、Namespace/MCP、LocalShell 结果闭环设为 TS-14 强制停点。
- R3：在 decoder/reconciler 接线前冻结历史 unbound 恢复合同。
- R4：用生产 Event Store 验证响应作用域身份和重启 replay。
- R5：完成 lifecycle invariant 重构后再关闭终态结算单元。
- R6：用实际生成的完整 ToolSpec 做 Provider acceptance。

##### Missing Tests

- R1/R2：生产 schema、Namespace/MCP identity、ToolSearch 加载闭环、LocalShell typed action、approval/cancel。
- R3/R4：历史 unbound 恢复、跨响应身份冲突、重启 replay。
- R5：终态后迟到结果和 `outcome_unknown`。
- R6：完整容器 Provider acceptance、顶层绕过拒绝、Standard final-wire 零差异。

##### Missing Logs / Observability

- LOG-1：接线时需要 `container_missing_with_hosted`、typed decode failure、unknown ToolSpec variant、outer/internal pairing
  failure、unbound backlog、identity conflict、late settlement after terminal 和 Standard changed-set；不得记录业务正文。

##### Evidence

- `third_party/codex-cli/codex-rs/core/src/tools/taskspace_sequence_schema_contract_tests.rs:6` - 测试内候选 envelope。
- `third_party/codex-cli/codex-rs/core/src/tools/nested_call.rs:8` - 当前 builder 只覆盖 Function/Freeform。
- `docs/v0.0.5/build-R8/tool-sequence-protocol/00-product-definition.md:169` - 历史 unbound 后续绑定要求。
- `third_party/codex-cli/codex-rs/core/src/action_map/event_store.rs:138` - 当前持久化归属入口。
- `third_party/codex-cli/codex-rs/core/src/action_map/rooted_dag/invariants.rs:263` - reservation/lifecycle 当前约束。
- `scripts/taskspace-benchmark/r8_hosted_container_probe.py:61` - 真实探针使用的是简化 function schema。
- [DeepSeek Responses API](https://api-docs.deepseek.com/api/create-response/) - 当前支持 function/web_search descriptor，
  `function_call_output.output` 为字符串，且要求客户端验证模型生成参数。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| test-validity-adversary | R1 candidate fixture 被表述为完整生产合同 | non-blocking | accept | 测试确实自定义 candidate types | 08 与计划改称“方向可行性”；TS-11 要求测试迁移到生产类型 | TS-11 |
| test-validity-adversary | R2 typed Tool 闭环未验证 | non-blocking | accept | `nested_call` 当前只覆盖 Function/Freeform | TS-14 验收扩展到 MCP/Namespace/ToolSearch/LocalShell 输入与结果 | TS-14 |
| test-validity-adversary | R3 历史 unbound 恢复合同缺失 | non-blocking | accept + defer design | 产品要求存在，但恢复载体未经用户决策 | 文档明确为 decoder/reconciler 前独立停点，不擅自选择 carrier | TS-12 前决策 |
| test-validity-adversary | R4 Hosted identity 未通过生产 Store replay | non-blocking | accept | 当前 binding fixture 使用测试 ledger | TS-12 增加响应作用域身份、冲突和重启 replay 验收 | TS-12 |
| test-validity-adversary | R5 终态迟到结果未验证 | non-blocking | accept | characterization test 仅复现当前拒绝 | TS-16 增加终态后迟到结果持久化验收 | TS-16 |
| test-validity-adversary | R6 最终 ToolSpec 未经 Provider acceptance | non-blocking | accept | 真实探针仅为简化 function schema | TS-11 加 final-wire 和 Provider acceptance；真实调用另行申请预算 | TS-11 |
| test-validity-adversary | B1 成本、质量、遵循率未测 | benefit warning | defer | 生产容器尚未接线，提前测量无效 | 保留为 Phase D 真实产品验证 | TS-22 |
| test-validity-adversary | LOG-1 生产可观测性缺口 | non-blocking | accept | 当前尚未进入生产接线 | 由既有 TS-17 日志单元承接 reviewer 列出的机械事件 | TS-17 |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: n/a
- Blocking re-review completed: n/a
- Blocking re-review passed: n/a
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Implementation completeness gaps resolved or accepted by user: n/a for feasibility scope
- Target benefit warnings recorded: yes
- Blocked reason: n/a
- Allowed to proceed: yes, with Phase B stop points

## Final Conclusion

对抗性审查无法构造会推翻薄容器方向的方案级反例。方向可行性通过，可以进入 Phase B；不得把该结论扩张为最终生产
schema、全部 typed Tool、恢复路径、持久化正交性或生产收益已经端到端验证。R1～R6 已转化为 Phase B 明确停点。
