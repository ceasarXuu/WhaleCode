# Problem P-001: R7 FLA map-request 稳定多 Patch 与 Map 使用退化
- Status: open
- Created: 2026-07-23 18:24
- Updated: 2026-07-23 18:24
- Objective: 区分 map-request 复杂样本中稳定出现的 multi-patch、偶发长尾和 Map 生命周期事后补账，并确认是否存在反馈丢失或可证明的 policy 因果。
- Symptoms:
  - map-request 复杂样本 3/3 在同一 provider response 生成多个 `apply_patch`，均被 one-patch preflight 原子拒绝。
  - complex repeat 3 使用 25 个 requests，并在工作完成后用 4 次 `echo` 事后闭合 Map。
- Expected behavior:
  - Agent 每个 response 最多生成一个可包含多文件的 `apply_patch`；真实动作发生时自然推进 Map，而不是完成工作后补账。
- Actual behavior:
  - Agent 把独立文件修改拆成 2 至 3 个 sibling `apply_patch`；一个长尾运行始终对 `explore_repo` 使用 `continue_current`，最终才读取 Map 并补做节点交接。
- Impact:
  - multi-patch 每次稳定浪费一个 provider request；长尾运行进一步增加请求、input、output 和 wall time。
- Reproduction:
  - FLA-8 `subscription-billing-repair`、`map-request`、repeat 1/2/3。
- Environment:
  - Linux/Docker，DeepSeek V4 Flash，subject commit `f2baea6d13caef02f15e1a3c6938a3fa05a3d315`，run `20260723-073642-091`。
- Known facts:
  - 3/3 multi-patch response 的普通工具执行数均为 0，状态提交保持原子。
  - `apply_patch` description 已明确一个 response 最多一个 call，并允许一个 call 修改多文件。
  - 三种 TaskSpace policy 使用相同 L1、L2 和 Tool schema；map-request 的区别是普通请求不自动携带当前 projection。
  - exact payload scan 609/609 通过，retention coverage 100%，semantic replacement 0。
  - repeat 3 的 action 始终携带正确 `current_node_id=explore_repo` 和 revision，不能归因为状态反馈丢失。
- Ruled out:
  - Runtime 执行了部分 patch 后再拒绝。
  - one-patch 规则没有暴露给 Agent。
  - TaskSpace control/ordinary feedback 在 provider context 中丢失或被改写。
- Fix criteria:
  - 通过同版本受控实验区分 projection 可见性、L2 并行措辞和 per-tool carrier 形状对 multi-patch 选择率的贡献；修复后自然复杂样本 multi-patch 为 0，Map 事后补账不增加，同时保留多工具连续动作能力。
- Current conclusion: multi-patch 的直接机制和额外 request 归因已确认：跨调用数量约束无法由单个 function arguments schema 表达，Agent 生成多个分别合法的 patch call 后才被 request-wide preflight 拒绝。为什么该选择集中出现在 map-request，当前只有强关联而没有严格因果证明。Map 事后补账不是 3/3 同形错误：repeat 2 路径正常，repeat 1 有重复测试式交接，repeat 3 才出现严重 echo 补账；其共同风险是 map-request 依赖 Agent 主动读取外部 Map，而 `continue_current` 可在机械上无限合法。没有证据支持反馈丢失或 Runtime 语义纠正。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: multi-patch 是跨调用合同无法由单 Tool schema 前置表达的稳定冲突
- Status: confirmed
- Parent: P-001
- Claim: Agent 生成的每个 `apply_patch` arguments 都符合单调用 schema，但同一 response 的 call 数违反 request-wide one-patch 合同，只能在生成后由 sequence preflight 发现并零执行拒绝。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - JSON Schema 只能校验一个 function call 的 arguments，不能约束 provider response 中同名 sibling call 数量。
- Falsifiable predictions:
  - If true: 3 次失败 response 都包含 2 至 3 个单独合法 patch call，preflight 在 dispatch 前拒绝全部调用。
  - If false: patch arguments 本身非法，或 Runtime 已部分执行后才失败。
- Diagnostic evidence plan:
  - Prediction or clause under test: 失败发生在 request-wide manifest preflight，且 executed count 为 0。
  - Signal: provider call array、failure code、executed_tool_call_count、state commit。
  - Capture method: 对 trace analysis、rollout 和 sequence preflight 代码逐项对账。
  - Event name or marker:
    - `request_multiple_apply_patch_calls_not_allowed`
  - Correlation keys:
    - repeat
    - request index
    - call id
  - Differentiates from:
    - patch parser/prepare failure
    - feedback loss
  - Supports if:
    - 所有 sibling 均收到同一 protocol failure，且执行数为 0。
  - Refutes if:
    - 任一 patch 已执行或失败发生在参数解析层。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - retain as permanent observability
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed；repeat 1/2/3 分别声明 2/3/2 个 patch call，全部由 request-wide preflight 原子拒绝。
- Repair design readiness: ready for the direct mechanism; causal policy attribution remains blocked on H-002
- Next step: 不改变 Runtime 原子拒绝边界；先做 H-002 的受控 provider experiment，再决定 provider-visible contract 修复点。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: map-request 缺少持续 projection 导致模型更倾向拆分 patch
- Status: unverified
- Parent: P-001
- Claim: 缺少当前 Map projection 降低了“当前 Work 是一个连贯工作单元”的持续显著性，使模型更倾向按文件生成多个 sibling patch，而不是合并为一个多文件 patch。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - 当前首轮 map-request complex 为 3/3 multi-patch，而 map-always/map-append 合计 0/6；三臂的 L1/L2/Tool schema 相同，主要可见差异是 projection policy。
- Falsifiable predictions:
  - If true: 在同一模型、同一已读文件、同一 patch 目标和同一 schema 下，只切换 current projection 可见性会显著改变 multi-patch 选择率。
  - If false: 有无 projection 的 multi-patch 选择率相近，或差异由随机路径、prompt 历史和 patch prepare 恢复解释。
- Diagnostic evidence plan:
  - Prediction or clause under test: projection 可见性是否是 multi-patch 选择率的因果变量。
  - Signal: provider response 中 patch call count、单 patch文件数、相同前置历史 hash。
  - Capture method: 构造不迎合结论的真实多文件修复快照，冻结 L1/L2/schema/历史，只切换当前 projection，并执行足够重复。
  - Event name or marker:
    - `provider.tool_call_batch`
    - `request_multiple_apply_patch_calls_not_allowed`
  - Correlation keys:
    - experiment arm
    - seed/run
    - history hash
  - Differentiates from:
    - H-001 的跨调用表达缺口
    - 单次 Agent 随机性
  - Supports if:
    - map-request 显著更高且置信区间与 always/append 分离。
  - Refutes if:
    - 受控实验无稳定差异。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 实验后只保留脱敏聚合结果和 schema/history hash。
- Evidence gate: pending
- Related evidence:
  - E-001
  - E-003
- Conclusion: unverified；当前 3/3 对 0/6 是强关联，但自然 rollout 的前置路径不同，不能直接宣称 projection policy 是因果根因。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 执行只改变 projection 可见性的受控实验。
- Blocker:
  - 尚无同历史、同目标的单变量 A/B。
- Close reason:
  - not closed

## Hypothesis H-003: 25-request 长尾由反馈丢失导致 Agent 不知道当前 Map 状态
- Status: refuted
- Parent: P-001
- Claim: repeat 3 长期停留在 `explore_repo` 并事后补账，是因为普通 Tool 或 control 反馈没有正确进入 provider context。
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - Agent 在工作完成后才发现多个节点未闭合，表面上像状态反馈缺失。
- Falsifiable predictions:
  - If true: provider payload 应缺少对应 call/result、revision 或 active node，或者出现 semantic replacement。
  - If false: Agent 每次都提交正确 node/revision，但自行选择持续 `continue_current`。
- Diagnostic evidence plan:
  - Prediction or clause under test: 状态身份和失败反馈是否逐请求忠实保留。
  - Signal: exact payload scan、retention coverage、taskspace action arguments、control result。
  - Capture method: 对 repeat 3 request 1 至 25 的 provider/rollout trace 对账。
  - Event name or marker:
    - `taskspace-exact-payload-scan-event-v1`
  - Correlation keys:
    - request index
    - call id
  - Differentiates from:
    - Agent 的 Map 使用选择
    - read_map discoverability
  - Supports if:
    - 状态反馈缺失、错配或被替换。
  - Refutes if:
    - 状态反馈完整且 action 使用正确 canonical identity。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - retain as permanent observability
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
- Conclusion: refuted；609/609 exact payload scan 通过，Agent 一直使用正确 `explore_repo` 和 revision。问题是选择持续使用 `continue_current`，不是不知道状态。
- Repair design readiness: not applicable
- Next step: 不通过 projection 重写、Runtime 语义判断或反馈强化来修复。
- Blocker:
  - none
- Close reason:
  - hypothesis refuted

## Evidence E-001: map-request complex 3/3 multi-patch 原子拒绝
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: reproduction
- Source: `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/f2baea6d13caef02f15e1a3c6938a3fa05a3d315/20260723-073642-091/trace-analysis.json`
- Prediction or plan link:
  - H-001/H-002：建立 multi-patch 频率、call 数和执行结果。
- Matched signal:
  - repeat 1 request 11 为 2 calls；repeat 2 request 8 为 3 calls；repeat 3 request 12 为 2 calls；均执行 0。
- Correlation keys:
  - sample `subscription-billing-repair`
  - arm `map-request`
- Raw content:
  ```text
  repeat 1: patch calls=2, failure=request_multiple_apply_patch_calls_not_allowed
  repeat 2: patch calls=3, failure=request_multiple_apply_patch_calls_not_allowed
  repeat 3: patch calls=2, failure=request_multiple_apply_patch_calls_not_allowed
  ```
- Interpretation: 证明当前样本下症状稳定，但不同自然 rollout 不能单独证明 map-request policy 因果。
- Time: 2026-07-23 18:24

## Evidence E-002: one-patch request-wide preflight 与 Tool description
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/tools/src/apply_patch_tool.rs:10-12`、`third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs:47-62`
- Prediction or plan link:
  - H-001：确认合同暴露和实际拒绝层。
- Matched signal:
  - Tool description 明确“一次 response 最多一个 apply_patch”；Runtime 统计整个 response 的 patch call 数并在大于 1 时零执行拒绝。
- Correlation keys:
  - failure code `request_multiple_apply_patch_calls_not_allowed`
- Raw content:
  ```text
  A provider response may contain at most one apply_patch call; include all related file operations in that one patch.
  provider response declares N apply_patch calls; maximum is 1; no tool calls were executed
  ```
- Interpretation: 排除“Agent 未被告知规则”；剩余问题是描述性跨调用合同的自然遵循率，而不是 Runtime 丢失规则。
- Time: 2026-07-23 18:24

## Evidence E-003: 三策略 schema 相同但首轮行为相关性不同
- Related hypotheses:
  - H-002
- Direction: supports
- Type: observation
- Source: `docs/v0.0.5/build-R7/35-r7-five-layer-fla8-initial-repeat3-result.md`、四臂 provider wire trace
- Prediction or plan link:
  - H-002：检查主要 policy 差异与 multi-patch 分布。
- Matched signal:
  - map-request complex 3/3 multi-patch；map-always/map-append complex 合计 0/6。三个 TaskSpace 臂 Tool section 均为 60,743 bytes、同一 hash。
- Correlation keys:
  - projection policy
- Raw content:
  ```text
  map-request: 3 multi-patch attempts / 3 complex runs
  map-always + map-append: 0 / 6
  tools schema: identical across three TaskSpace arms
  ```
- Interpretation: 将候选因果变量收敛到 projection 可见性或其引起的历史路径差异，但不足以越过受控因果门。
- Time: 2026-07-23 18:24

## Evidence E-004: repeat 3 状态身份完整但发生 read_map 误用和事后补账
- Related hypotheses:
  - H-003
- Direction: refutes
- Type: diagnostic-log
- Source: FLA trace analysis 与 repeat 3 rollout
- Prediction or plan link:
  - H-003：区分反馈缺失与 Agent 在完整反馈下的动作选择。
- Matched signal:
  - request 1 至 18 的 ordinary actions 持续使用正确 `explore_repo`/revision；request 6 把 `read_map` 错放进 ordinary action，request 7 幻觉为顶层 Tool，request 20 才正确调用 `taskspace_control.read_map`；随后 4 次 echo 交接。
- Correlation keys:
  - repeat 3
  - requests 6、7、19-25
- Raw content:
  ```text
  r6: exec_command + taskspace_action.read_map -> TASKSPACE_ACTION_INVALID
  r7: top-level read_map -> TASKSPACE_ACTION_REQUIRED
  r19: finish_map -> TASKSPACE_LIFECYCLE_INVARIANT
  r20: taskspace_control.read_map -> success
  r21-r24: complete_then_continue + echo
  r25: finish_map -> success
  ```
- Interpretation: 反馈链路忠实；长尾由 read_map 调用方式混淆、持续选择 `continue_current`、终态才对账以及软预算继续共同放大，不是上下文语义丢失。
- Time: 2026-07-23 18:24
