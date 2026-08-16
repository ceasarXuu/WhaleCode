# Problem P-001: Agent 高频提前选择 Waiting 后继节点
- Status: open
- Created: 2026-08-16 23:06
- Updated: 2026-08-17 00:30
- Objective: 解释 `single-file-fast-fix` 当前 TaskSpace Exec 合同下 Waiting frontier 误选约 40% 复发的发生机制与主要诱因，并在证据不足时阻止提示词或 Runtime 约束继续扩张。
- Symptoms:
  - client work 恢复 repeat=10 中，Run 6/8/9/10 在 `apply_patch@fix` 成功后的下一请求直接提交 `work(exec_command@verify)`。
  - `verify` 仍依赖未完成的 `fix`，Runtime 以 Waiting frontier 硬规则零副作用拒绝。
- Expected behavior:
  - Agent 在同一请求中使用 `update_and_work`，先显式将 `fix` 更新为 `completed`，再执行 `exec_command@verify`。
- Actual behavior:
  - 4/10 选择较简单的 `work` 分支；拒绝后 4/4 下一请求立即改为正确的 `update_and_work`。
- Impact:
  - 每次至少放大一个 Provider 请求和对应 input/output；当前业务正确性未受损，因为 Runtime 在副作用前守住 DAG 硬边界。
- Reproduction:
  - `single-file-fast-fix × map-request × repeat=10`，subject `560d07827`，二进制 SHA-256 `7351aafdccb2ad60409c7437163adebd072d2b43361b3640f9473ec2fe0824c2`。
  - 证据 roots：`target/r8-client-work-restoration/repeat10-{1..10}/single-file-fast-fix/*`。
- Environment:
  - branch `whalecode-alpha`；model `deepseek-v4-flash`；TaskSpace base `3.0.3`；Runtime capability identity `571f3cfe8d9e3686e95423330c0de1af45ea300d257b5af4146082981b7acbfe`。
- Known facts:
  - 当前 Tool description 和 `work` / `update_and_work` 分支 description 都明确写出 Tool outcome 不完成节点，以及需要先完成 parent 时使用 `update_and_work`。
  - 4 次误选都紧邻成功 `apply_patch`；6 次同位置正确选择 `update_and_work`。
  - Run 9 reasoning 明确知道 `fix` 仍为 `in_flight`，仍然选择 `work@verify`。
  - 4 次 Waiting reject 后，Agent 均准确解释未完成父节点并在下一请求修正。
  - 当前成功 feedback 写有 outer `status=completed` 和 client `outcome=succeeded`，但不返回 owner 的当前节点状态；完整 Map 只由 `read_map` 返回。
  - owner-state 单变量候选曾返回 `owner_state_after=in_flight`；四轮到达 patch-to-verify 边界仍有 2/4 误选，候选已回退。
  - repeat=10 无 cache-shape、Tool choice 或 capability identity 切换，也无 Provider retry。
- Ruled out:
  - Runtime 错误计算 `verify` readiness。
  - Waiting reject 反馈丢失、错误分类或语义扭曲。
  - Agent 完全不知道 `fix` 尚未 completed。
  - 当前 Tool schema 完全缺少 handoff 规则。
- Fix criteria:
  - 先用单变量实验区分“结果反馈缺少 owner state”和“闭集分支选择/命名不显著”两类诱因；未经该证据不得追加提示词或 Runtime 自动完成节点。
  - 候选修复必须保持 Tool outcome 不自动完成节点、Agent 决定 lifecycle、Runtime 只守 DAG 硬边界。
  - 后续真实复验需证明 Waiting 频率和请求成本下降，且不引入冗余状态转换、Map 坍缩、缓存或 Standard 回归。
- Current conclusion: 已确认的直接机制是：动态 Map frontier 不能由静态 Function schema 表达，`work` 与 `update_and_work` 在 Provider schema 层都可生成；Agent 在自然的“补丁成功后立刻测试”路径中会省略 lifecycle handoff，Runtime 到动态 preflight 才能拒绝。单变量 owner-state 反馈在实际到达边界的四轮中仍误选 2/4，且两次 trace 均证明 Agent 已收到 `fix=in_flight`；该候选不通过并已回退。当前优先候选收敛为序列分支结构显著性，而不是继续增加反馈字段、提示文字或 Runtime 状态职责。
- Current conclusion: 已确认的直接机制是：动态 Map frontier 不能由静态 Function schema 表达，`work` 与 `update_and_work` 在 Provider schema 层都可生成；Agent 在自然的“补丁成功后立刻测试”路径中会省略 lifecycle handoff，Runtime 到动态 preflight 才能拒绝。单一 `owner_state_after` 不是 Waiting 误选的充分修复，但不能由此推导“Runtime 机械改变的 canonical 状态无须在反馈中可见”。新候选与旧实验严格区分：不再堆叠孤立 owner 字段，而是忠实返回本批次直接操作或机械变更的节点状态，并对仍 Waiting 的直接 Work 子节点返回精确未完成父节点。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
- Resolution basis:
  - direct mechanism satisfied；probability-inducing factors not fully isolated
- Close reason:
  - not closed

## Hypothesis H-001: 静态闭集只能约束形状，不能约束动态 frontier
- Status: confirmed
- Parent: P-001
- Claim: Provider Function schema 同时允许 `work` 与 `update_and_work` 的合法形状，但无法将当前 Map 中 `verify=waiting` 编入 schema；因此错误的 `work@verify` 会通过模型侧结构生成，只能由 Runtime 动态 preflight 拒绝。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - `node_id` 是 Agent 生成的字符串，当前节点状态来自独立 canonical Map，静态 JSON schema 没有当前 Map 状态输入。
- Falsifiable predictions:
  - If true: 错误请求应具有合法 `work` 结构，并只在 Runtime 读取当前 Map 后因 owner Waiting 被拒绝。
  - If false: Provider schema 本身应拒绝该结构，或 Runtime 在 decode 前即可由静态字段证明 `verify` 不可执行。
- Diagnostic evidence plan:
  - Prediction or clause under test: 错误结构通过 typed decode，在 dynamic preflight 的 `ClientNodeNotExecutable` 分支失败。
  - Signal: 原始 Function arguments、sequence schema、preflight error。
  - Capture method: 对照 4 个 rollout 与 `sequence_schema.rs`、`preflight.rs`、`handler.rs`。
  - Event name or marker:
    - `taskspace.exec.rejected / preflight_rejected`
  - Correlation keys:
    - outer call ID
  - Differentiates from:
    - H-004 schema 规则缺失
  - Supports if:
    - 四次均 decode 为 `type=work` 且拒绝原因为 `verify waiting / incomplete parent fix`。
  - Refutes if:
    - 任一次因 JSON/schema decode 失败，或 `fix` 已 completed。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: 当前闭集结构能防止未知形状，不能在 Provider 生成时表达 request-time Map frontier；动态硬门正确但必然是后置拒绝。
- Repair design readiness: blocked until probability-inducing factor is isolated
- Next step: 用最小单变量比较结果反馈或分支显著性，不扩大 Runtime 状态职责。
- Blocker:
  - 用户尚未批准修复实验和真实运行预算。
- Close reason:
  - not closed

## Hypothesis H-002: 自然 coding flow 与较短 `work` 分支共同形成选择偏置
- Status: confirmed
- Parent: P-001
- Claim: 在补丁成功后，Agent 的直接行动目标统一变成“运行测试”；当前 `work` 能以更短结构表达该动作，4/10 因此省略父节点 completion，而不是缺少测试计划或误选 Tool。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - Standard coding prior 是 patch 后直接 test；TaskSpace 额外要求在同一调用中携带 lifecycle handoff。
- Falsifiable predictions:
  - If true: 所有误选都应发生在成功 patch 后，reasoning 均表达下一步测试，Tool 和 owner 选择本身正确，只缺 `fix:completed`。
  - If false: 误选会平均分布在读取、修改、验证等边界，或选择错误 Tool/错误 owner。
- Diagnostic evidence plan:
  - Prediction or clause under test: 误选位置和缺失字段具有一致模式。
  - Signal: 成功 patch output 后第一条 reasoning 与第一条 Function Call。
  - Capture method: 对十个 rollout 做相同索引提取。
  - Event name or marker:
    - successful `apply_patch` client result
  - Correlation keys:
    - run number、outer call ID
  - Differentiates from:
    - 随机 Map corruption 或错误 Tool 选择
  - Supports if:
    - 4/4 都是 patch success -> run tests -> `work@verify`，6/6 对照是 patch success -> `update_and_work(fix completed)+verify`。
  - Refutes if:
    - 失败位置或缺失结构不一致。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-012
- Conclusion: 错误不是无目标的随机动作，而是自然下一 Tool 行动压过了 Map handoff 记账；这解释模式一致性，但尚不能单独证明“分支更短”是模型选择的因果变量。
- Repair design readiness: implemented offline; live validation pending
- Next step: 获得真实运行预算后验证组合协议修复是否降低 Waiting frontier；本轮同时修复生命周期合同和断裂示例，不能把
  在线差异单独归因给其中一个因素。
- Blocker:
  - 缺少当前候选的真实 Agent 行为证据。
- Close reason:
  - not closed

## Hypothesis H-003: 成功 feedback 的作用域不清和 owner state 省略提高误选概率
- Status: closed
- Parent: P-001
- Claim: outer `status=completed`、client `outcome=succeeded` 强调 Tool/Exec 完成，而 feedback 不携带 `fix` 仍为 `in_flight` 的事实，使局部上下文更容易触发“下一步直接测试”。
- Layer: sub-cause
- Factor relation: any_of
- Depends on:
  - H-001
- Rationale:
  - 当前 result contract 只有 read_map 结果携带完整节点状态；普通 work feedback 没有 owner post-state。
- Falsifiable predictions:
  - If true: 在保持 schema、Map、任务和模型不变时，增加机械且忠实的 owner post-state 会降低 Waiting 误选。
  - If false: 即使 Agent 明确看到或准确记得 `fix=in_flight`，仍以相近频率选择 `work@verify`。
- Diagnostic evidence plan:
  - Prediction or clause under test: owner state 可见性是选择差异变量。
  - Signal: 同一 sample 的 baseline 与仅反馈增加 owner post-state 的 Waiting 频率、请求数和 reasoning。
  - Capture method: 先做离线 result-contract fixture；真实 A/B 需另行预算。
  - Event name or marker:
    - `client_results[].owner_state_after`
  - Correlation keys:
    - outer call ID、node_id
  - Differentiates from:
    - H-002 分支选择偏置、H-005 Base 原则歧义
  - Supports if:
    - 单变量反馈使 Waiting 明显下降且无新增误操作。
  - Refutes if:
    - 频率不变或 Agent 仍明确知道 state 后误选。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-004
  - E-005
  - E-008
  - E-009
- Conclusion: 反馈省略是事实，但补充机械 owner state 后四个有效边界仍有两次误选；两次失败均逐字收到 `owner_state_after=in_flight`。当前数据不支持把该字段作为 Waiting 修复，且已直接证明状态省略不是必要或充分根因。小样本不能排除极小概率贡献，但该分支因无实际收益而关闭。
- Repair design readiness: not applicable
- Next step: 回到 H-002 的 schema 分支显著性单变量，不继续扩展 feedback。
- Blocker:
  - none
- Close reason:
  - controlled candidate failed acceptance and was reverted

## Hypothesis H-004: Waiting 规则或拒绝反馈缺失
- Status: refuted
- Parent: P-001
- Claim: Agent 误选是因为 Tool schema 没有说明 handoff，或 reject 没有准确告诉 Agent 未完成父节点和正确恢复方向。
- Layer: sub-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 历史上该规则曾经分散，可能继续存在暴露缺口。
- Falsifiable predictions:
  - If true: final Tool description 不包含 prior Tool outcome、parent completion 或 `update_and_work`；拒绝后 Agent 继续重复相同错误。
  - If false: schema 同时在全局与 branch 层暴露规则，拒绝后一次纠正。
- Diagnostic evidence plan:
  - Prediction or clause under test: 最终源码规则和在线恢复行为。
  - Signal: `protocol.rs`、`sequence_schema.rs`、四次 reject 后下一调用。
  - Capture method: 静态读取和 rollout 对照。
  - Event name or marker:
    - `ClientNodeNotExecutable`
  - Correlation keys:
    - outer call ID
  - Differentiates from:
    - H-001/H-002
  - Supports if:
    - 规则缺失或 Agent 重复误选。
  - Refutes if:
    - 规则存在且 4/4 下一请求正确恢复。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-006
- Conclusion: 规则和拒绝反馈均存在且在线有效；继续增加同义文字不是证据支持的主修复方向。
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - refuted by current source and trace

## Hypothesis H-005: Base 的低频 lifecycle 原则与单 Tool 节点模型存在局部歧义
- Status: unverified
- Parent: P-001
- Claim: Base 要求在 meaningful work boundary 更新 lifecycle、不要在每个 minor Tool result 后更新；在本 sample 的 `fix` 节点基本由一次 patch 完成时，Agent 可能把 patch success 当作无需更新的 minor result，从而直接进入验证。
- Layer: interaction
- Factor relation: any_of
- Depends on:
  - H-002
- Rationale:
  - 宏观原则旨在防止每个命令都成为 Map 负担，但节点粒度与 Tool 粒度接近时存在解释空间。
- Falsifiable predictions:
  - If true: 移除或精确收敛该句、保持 Tool schema 不变，会降低 fix->verify Waiting 误选而不增加微粒度节点更新。
  - If false: 单变量修改后误选率不变，或 Agent 当前 reasoning 显示采用了完全不同的判断。
- Diagnostic evidence plan:
  - Prediction or clause under test: Base 原则是否改变节点边界行动。
  - Signal: 单变量 final-wire hash、相同 sample trace、节点数量与 Waiting 频率。
  - Capture method: 仅在 H-003/分支显著性候选不能解释时考虑；真实运行需预算。
  - Event name or marker:
    - TaskSpace base version/hash
  - Correlation keys:
    - request shape、run ID
  - Differentiates from:
    - H-003 反馈局部作用域
  - Supports if:
    - Waiting 下降且 Map 粒度不退化。
  - Refutes if:
    - 无收益或引入过度记账。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: pending
- Related evidence:
  - E-007
- Conclusion: 存在文字上的可解释歧义，但没有 trace 直接表明 Agent 因该句跳过 completion；优先级低于结构/反馈候选。
- Repair design readiness: blocked until higher-priority hypotheses are tested
- Next step: 暂不修改 Base。
- Blocker:
  - 缺少因果证据。
- Close reason:
  - not closed

## Evidence E-001: 四次错误均通过结构 decode 后在动态 preflight 拒绝
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: repeat10 Run 6/8/9/10 `rollout.jsonl`
- Prediction or plan link:
  - H-001 dynamic preflight prediction
- Matched signal:
  - `type=work`、`exec_command@verify`；`verify` state `waiting`；incomplete direct parent `fix`
- Correlation keys:
  - `call_00_vh5aqLRmN3yFn3GuU28P6522`
  - `call_00_nNJ3JmxAOlyW0EDHRKYG6965`
  - `call_00_OXHGWKPQDEyWsI4dZ7T10833`
  - `call_00_WV4ZZJUO99L2MuKIZuOr2366`
- Raw content:
  ```text
  Tool action 0 targeted work node `verify` in state `waiting`; incomplete direct parent nodes: ["fix"]. Only the sequence's preceding Map operation can unlock work; Tool outcomes do not change node state. No Map or Tool actions were executed.
  ```
- Interpretation: 错误不是 JSON/schema 形状失败，而是 schema 无法携带的当前 Map 状态约束失败。
- Time: 2026-08-16 23:06

## Evidence E-002: Provider-visible schema 同时保留 work 与 update_and_work
- Related hypotheses:
  - H-001
  - H-004
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/tools/taskspace_exec/sequence_schema.rs:51`
- Prediction or plan link:
  - H-001 schema capability；H-004 rule presence
- Matched signal:
  - `work` 只要求非空 `tools`；动态 owner state 由 preflight 判断；`update_and_work` 额外要求 `update_map`
- Correlation keys:
  - capability identity `571f3cfe...1b7acbfe`
- Raw content:
  ```text
  work: every owner is already Ready or InFlight ... use update_and_work instead
  update_and_work: Update the Map first ... complete or change parent nodes before working on direct dependents
  ```
- Interpretation: 两种结构都必须存在以覆盖不同产品场景；静态 schema 只能描述适用条件，不能从 current Map 消除不适用分支。
- Time: 2026-08-16 23:06

## Evidence E-003: 十轮 patch 后的下一行动形成 6/4 单一分叉
- Related hypotheses:
  - H-002
- Direction: supports
- Type: reproduction
- Source: repeat10 十个 `rollout.jsonl` 的 successful apply_patch 后第一条 reasoning/Function Call
- Prediction or plan link:
  - H-002 位置和缺失结构一致性
- Matched signal:
  - Run 1/2/3/4/5/7：`update_and_work(fix:completed) + exec_command@verify`
  - Run 6/8/9/10：`work + exec_command@verify`
- Correlation keys:
  - run 1..10
- Raw content:
  ```text
  All ten reasonings reduce to: patch applied/succeeded; now run tests.
  The four rejected calls differ only by omitting update_map(fix -> completed).
  ```
- Interpretation: 错误受同一个自然工作边界触发；Tool、owner 和业务下一步均正确，只有 lifecycle handoff 被省略。
- Time: 2026-08-16 23:06

## Evidence E-004: 普通 work feedback 不返回 owner 节点 post-state
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/tools/taskspace_exec/result.rs:13`
- Prediction or plan link:
  - H-003 feedback shape fact
- Matched signal:
  - result fields包含 outer `status=completed`、client `outcome`、`node_id` 和 Tool result；只有 `reads[].map` 携带节点 state
- Correlation keys:
  - result kind `taskspace_exec_result`
- Raw content:
  ```text
  TaskSpaceExecResult { status, map_revision_at_dispatch, reads, client_results }
  ClientResult { node_id, tool, outcome, result, error, settlement_error }
  ```
- Interpretation: 反馈省略是确定事实；它是否导致 4/10 误选仍需单变量验证。
- Time: 2026-08-16 23:06

## Evidence E-005: Run 9 明知 fix=in_flight 仍直接测试
- Related hypotheses:
  - H-003
  - H-002
- Direction: refutes
- Type: observation
- Source: repeat10 Run 9 `rollout.jsonl`
- Prediction or plan link:
  - H-003 充分根因反证
- Matched signal:
  - reasoning 明确写出 owner state，随后仍选择 `work@verify`
- Correlation keys:
  - `call_00_OXHGWKPQDEyWsI4dZ7T10833`
- Raw content:
  ```text
  The fix node is in_flight now (since a tool action on it was executed). Let me run the tests.
  ```
- Interpretation: 显式状态知识没有自动转化为合法序列选择；补充 owner state 可能有帮助，但不能被视为已坐实的完整修复。
- Time: 2026-08-16 23:06

## Evidence E-006: 当前规则清晰且四次均一次恢复
- Related hypotheses:
  - H-004
- Direction: refutes
- Type: observation
- Source: `protocol.rs`、`sequence_schema.rs`、repeat10 Run 6/8/9/10
- Prediction or plan link:
  - H-004 schema/feedback absence prediction
- Matched signal:
  - 全局和 branch 说明均存在；reject 后四次均提交 `update_and_work(fix:completed)+verify`
- Correlation keys:
  - four Waiting outer call IDs
- Raw content:
  ```text
  Tool outcomes do not complete nodes.
  If a Map update must complete or change a parent first, use update_and_work instead.
  ```
- Interpretation: 反馈准确性和恢复能力已通过；当前问题不是继续解释拒绝，而是拒绝前如何稳定选择合法分支。
- Time: 2026-08-16 23:06

## Evidence E-007: Base 要求 lifecycle 更新保持在 meaningful boundary
- Related hypotheses:
  - H-005
- Direction: neutral
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/whalecode_taskspace.md:62`
- Prediction or plan link:
  - H-005 文本存在性与解释空间
- Matched signal:
  - 同一句同时要求 keep Map aligned 和避免每个 minor Tool result 后更新
- Correlation keys:
  - TaskSpace base version `3.0.3`
- Raw content:
  ```text
  Keep it aligned as your understanding changes, and update lifecycle state at meaningful work boundaries rather than after every minor Tool result.
  ```
- Interpretation: 该原则产品方向正确，但一次 patch 是否属于 meaningful boundary 由 Agent 解释；当前 trace 没有直接引用该原则，故仅为待验证交互因素。
- Time: 2026-08-16 23:06

## Evidence E-008: owner state 候选只增加忠实 feedback
- Related hypotheses:
  - H-003
- Direction: supports
- Type: focused-test
- Source: candidate `96254de81`；`taskspace_exec_handler_tests`
- Prediction or plan link:
  - H-003 单变量实现边界
- Matched signal:
  - 每条 client result 必填复制 persisted candidate Map 的 owner state；schema、Base、DAG、拒绝和 Tool 执行逻辑不变
  - TaskSpace Exec 聚焦测试 67 passed；缓存门禁通过
- Correlation keys:
  - candidate binary SHA-256 `5cf9d12d...f6ae`
- Raw content:
  ```text
  client_results[].owner_state_after = canonical owner state after the batch
  ```
- Interpretation: 真实运行中的差异可归因于 feedback owner state，而不是其他序列或 Runtime 规则改动。
- Time: 2026-08-16 23:50

## Evidence E-009: 显式 owner state 后 Waiting 仍为 2/4
- Related hypotheses:
  - H-003
  - H-002
- Direction: refutes
- Type: fix-validation
- Source: `target/r8-owner-state-feedback/repeat5-{1..5}/single-file-fast-fix/*`
- Prediction or plan link:
  - H-003 增加 owner state 应降低 Waiting 误选
- Matched signal:
  - Run 1/3 的 patch success feedback 均逐字包含 `fix + owner_state_after=in_flight + outcome=succeeded`
  - 两轮下一 reasoning 仍为 patch applied -> run tests，并提交 `work@verify`
  - Run 2/4 正确；Run 5 因独立顶层 client Tool 逃逸未到达目标边界
- Correlation keys:
  - Run 1 `call_00_poaU1mHXDwweh9urdfwX8395`
  - Run 3 `call_00_ARW8Ox8ImjsfftndjA9k9977`
- Raw content:
  ```text
  eligible frontier transitions: 4
  FRONTIER-EARLY: 2/4
  historical current baseline: 4/10
  ```
- Interpretation: 字段已进入上下文但没有改变两次错误行动；候选未观察到下降，不能晋升。实现由 `52d209637` 回退。
- Time: 2026-08-16 23:56

## Hypothesis H-006: 批次状态反馈不完整使 Agent 缺少连续 canonical Map 事实
- Status: closed
- Parent: P-001
- Claim: 旧候选只回传 client owner 的一个 post-state，未返回本批次直接操作节点、Runtime 机械变更节点和仍不可执行直接 Work 子节点的精确依赖事实；这个不完整反馈使 Agent 对 Map 状态的掌握不连续。
- Layer: feedback
- Factor relation: any_of
- Depends on:
  - H-001
- Rationale:
  - `map-request` 不在每次 Provider request 自动展开完整 Map；Runtime 又会机械执行 readiness 和 activation 转换。成功 Tool feedback 应忠实回传本批次相关 canonical 状态，不要求 Agent 从分散历史推测。
- Falsifiable predictions:
  - If true: 在不改变状态机、序列闭集、Base 和 Tool 执行的前提下，返回 affected node states 与精确 unavailable child facts 应至少使 Agent 在 trace 中稳定获得完整状态事实，并可能降低 Waiting frontier 误选。
  - If false: 反馈字段未出现、与 canonical Map 不一致，或 Agent 在确认收到完整相关事实后仍以相近频率误选。
- Diagnostic evidence plan:
  - Prediction or clause under test: 批次反馈完整性和 Waiting 行为收益。
  - Signal: `affected_node_states[]`、`unavailable_direct_work_children[]`、canonical Map 对照、Waiting reject 频率、请求/token/cache/耗时。
  - Capture method: 聚焦 Rust 测试先证明输出忠实；然后 `single-file-fast-fix × map-request × repeat=5`，零自动重试。
  - Event name or marker:
    - `taskspace_exec_result`
    - `taskspace.exec.completed`
    - `taskspace.exec.rejected / preflight_rejected`
  - Correlation keys:
    - outer call ID、node ID、Map revision
  - Differentiates from:
    - H-003 孤立 owner post-state；H-002 序列分支选择偏置
  - Supports if:
    - 本批次相关状态与未完成父节点逐字进入 Agent 上下文，且无错误语义或状态机变更；行为收益另行统计，不以单轮成功宣称因果。
  - Refutes if:
    - 反馈与已持久化 Map 不一致，或精确事实未出现在 Tool output。
  - Instrumentation status: candidate feedback only
  - Instrumentation lifecycle:
    - 反馈字段是产品候选，非诊断日志；只有忠实性与回归通过才可保留。
- Evidence gate: satisfied
- Related evidence:
  - E-004
  - E-008
  - E-009
  - E-010
  - E-011
- Conclusion: 反馈缺口已由 canonical affected-state 结果补全，五轮均证明字段忠实进入上下文；但 Agent 在明确收到
  `fix=in_flight`、`verify=waiting` 和未完成父节点后仍有 2/5 误选，与 4/10 基线同率。该缺口是产品反馈完整性问题，
  不是 Waiting frontier 的充分根因。
- Repair design readiness: not applicable
- Next step: I04 回到 H-002 的合法序列分支选择，不继续堆叠同义状态反馈。
- Blocker:
  - none
- Close reason:
  - feedback fidelity fixed; causal prediction for Waiting behavior refuted

## Evidence E-010: 用户确认状态反馈产品边界并批准复验
- Related hypotheses:
  - H-006
- Direction: supports
- Type: user-feedback
- Source: 2026-08-17 当前任务
- Prediction or plan link:
  - H-006 修复授权和输出语义
- Matched signal:
  - 用户确认 `taskspace_exec` 应返回操作过的 node 状态，并允许对非 completed 节点补充精确下游不可执行事实；批准 `repeat=5` 真实运行预算。
- Correlation keys:
  - `USER-20260817-AFFECTED-STATE-R5`
- Raw content:
  ```text
  按你的建议试一下这两点改动，然后repeat 5 测试，批准预算
  ```
- Interpretation: 修复与付费复验已获明确授权；不包含新序列、状态机改动或 Runtime 自动完成节点。
- Time: 2026-08-17 00:30

## Evidence E-011: 完整 affected-state 反馈未降低 Waiting frontier 误选
- Related hypotheses:
  - H-006
  - H-002
- Direction: refutes
- Type: fix-validation
- Source: `target/r8-feedback-candidate/repeat5-{1..5}/single-file-fast-fix/*`
- Prediction or plan link:
  - H-006 批次反馈完整性和 Waiting 行为收益
- Matched signal:
  - 五轮成功结果均返回 canonical `affected_node_states`；patch 后逐字给出 `fix=in_flight`、`verify=waiting`、
    `incomplete_parent_ids=["fix"]`
  - Run 3/4 下一请求仍提交 `work(exec_command@verify)`，均被零副作用拒绝
  - 五轮业务、验证和 Map 为 5/5；FRONTIER-EARLY 为 2/5，与当前 4/10 基线同率
- Correlation keys:
  - Run 3 `call_00_wXhQtX3eITvLAvmGwxec9963`
  - Run 4 `call_00_18fwKFHLnRohacwwBTHV4223`
  - candidate `a36a42939`
- Raw content:
  ```text
  affected feedback present: 5/5
  business and Map closed: 5/5
  FRONTIER-EARLY: 2/5
  requests/input/cached/uncached/output: 44/676592/596352/80240/12676
  ```
- Interpretation: 反馈已忠实补全，排除字段未进入上下文或状态事实丢失；行为误选仍复发，故 H-006 不能作为 Waiting 的
  充分根因，I04 不关闭。
- Time: 2026-08-17 02:14

## Evidence E-012: Agent-visible lifecycle 合同与连续示例通过离线验收
- Related hypotheses:
  - H-002
- Direction: supports
- Type: focused-test
- Source: `taskspace_exec/protocol.rs`、catalog/preflight tests、R8 lifecycle 修复文档
- Prediction or plan link:
  - 在不增加 Runtime 状态约束的前提下，让 Tool description 完整表达已有状态机，并让示例形成真实合法路径
- Matched signal:
  - description 唯一包含完整 `Map lifecycle contract`
  - canonical 示例统一为 `root -> inspect -> implement -> finish`
  - 三个示例按顺序作用于同一 Map 后 Root、Implement、Finish 全部 Completed
  - TaskSpace Exec 70 tests、格式和免费 cache final-wire gate 通过
- Correlation keys:
  - cache candidate `e31a4ebf5d69087e02e3effe2b5a6e1b9b12716543a8e48487b5b4f6a5e7593e`
- Raw content:
  ```text
  canonical_examples_form_one_executable_lifecycle ... ok
  test result: ok. 70 passed; 0 failed
  ```
- Interpretation: 协议缺失与示例断裂已在工程层修复，未改变 Runtime 状态机；尚无真实 Agent 证据证明 Waiting 频率下降。
- Time: 2026-08-17 02:43

## Evidence E-013: 完整状态机协议未降低 Waiting frontier 误选

- Related hypotheses:
  - H-002
- Direction: refutes
- Type: fix-validation
- Source: `target/r8-state-machine-contract/repeat5-{1..5}/single-file-fast-fix/20260817-025457-*`
- Prediction or plan link:
  - 唯一完整状态机协议与连续 canonical 示例应降低 Waiting frontier 误选和非法状态转换
- Matched signal:
  - 5/5 业务、公开验证、隐藏 oracle 和 Map 闭合通过
  - Run 1/5 在收到 `fix=in_flight`、`verify=waiting` 后仍先执行 `work@verify`
  - Run 2/4 共出现 3 次同一 update 将进入序列时仍为 Waiting 的 child patch 为 InFlight 的 `TransitionInvalid`
- Correlation keys:
  - subject commit `7a798abdb`
  - ledger `WAR-20260817-025312-STATE-MACHINE-R5`
- Raw content:
  ```text
  FRONTIER-EARLY: 2/5
  TransitionInvalid: 3
  malformed JSON: 2
  requests/input/uncached/output: 41 / 639497 / 112009 / 11932
  ```
- Interpretation: 状态机协议与示例已进入 Agent 可见 description，但 Waiting 误选与上一轮同为 2/5，且最明确的
  same-update Waiting 边界仍被违反。信息完整性修复成立，行为收益预测不成立；不继续堆叠同义状态说明。
- Time: 2026-08-17 02:57
