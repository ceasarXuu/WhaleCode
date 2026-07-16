# Problem P-001: R6 TaskSpace 上下文与缓存成本高于 Standard
- Status: open
- Created: 2026-07-16 18:52
- Updated: 2026-07-16 19:15
- Objective: 定位并消除不必要的请求重复、Map 状态重复和 provider cache shape 变化，同时保持语义透传与 R6 correctness。
- Symptoms:
  - simple R6/Standard request=1.40x、input=1.52x、uncached=3.33x。
  - complex R6/Standard request=1.16x、input=1.35x、uncached=2.33x。
- Expected behavior:
  - TaskSpace 只增加 canonical DAG 所必需的上下文，当前 Map 状态只有一个权威载体，机械协议不破坏稳定缓存前缀。
- Actual behavior:
  - R6 增加 control call/result 和 provider request，并在 bootstrap/work/terminal 间改变 schema、tools 和 tool_choice。
- Impact:
  - R6 在结果正确时仍增加 token、未缓存输入和耗时，影响日常 coding agent 成本。
- Reproduction:
  - 运行 Phase E simple/complex Docker pair，各 Arm 3 次；读取 performance observation 和 provider cache trace。
- Environment:
  - Linux/Docker，branch `whalecode-alpha`，R6 runtime `0ce775278`，observer `84a35dbaa`，DeepSeek V4 Flash。
- Known facts:
  - Phase E 6/6 R6 run correctness、Map closure、terminal proof 通过。
  - 每个 provider payload exact scan 的 active projection count 为 1。
  - 每个 R6 run 有两次 tool choice/shape transition。
- Ruled out:
  - active projection 在 provider payload 中无限累加。
- Fix criteria:
  - payload section 可独立计量；当前完整 Map provider-visible owner=1；schema/choice transition=0；correctness/terminal/replay=100%；每项修复有独立成本对比。
- Current conclusion: H-001/H-002/H-003/H-004 均已确认；F1 必须先修复每请求 projection freshness，再删除 control `map_state`。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: 额外 provider request 反复携带增长历史
- Status: confirmed
- Parent: P-001
- Claim: R6 total input 增量的一部分由新增 request 对同一自然历史前缀的重复发送直接造成。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - Phase E aggregate 同时显示 request 与 input 放大，provider request 使用累计 history。
- Falsifiable predictions:
  - If true: 用 Standard 平均每 request input 乘 R6 额外 request，可解释显著比例的 total input delta。
  - If false: R6 与 Standard request 数接近，或额外 request 的上下文不包含已有前缀。
- Diagnostic evidence plan:
  - Prediction or clause under test: 额外 request 数对 input delta 的机械贡献。
  - Signal: request count、平均 input、LCP message bytes。
  - Capture method: 聚合 Phase E token summary 与 provider cache trace。
  - Event name or marker:
    - `TaskSpaceProviderCacheTraceV3`
  - Correlation keys:
    - run root、pair、model_request_index
  - Differentiates from:
    - H-002 每 request payload 自身更重
  - Supports if:
    - 额外 request 在 Standard 平均 request 成本下解释至少 25% input delta，且 LCP 保持。
  - Refutes if:
    - 解释比例低于 10% 或新增 request 不重发已有 message prefix。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - retain as permanent observability
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: confirmed；按 Standard 平均 request 成本估算，新增 request 分别解释 simple 76.22%、complex 46.23% 的 input delta。
- Repair design readiness: ready
- Next step: F0 记录稳定 section cost，F3 验证 Agent 声明动作合并。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: control 历史和 projection 重复表达当前 Map
- Status: confirmed
- Parent: P-001
- Claim: 每次成功 control result 的完整 `map_state` 与下一请求 active projection 重复表达当前 Map，且历史结果继续保留。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - canonical task context event 显示每次 control success 都携带完整 `map_state`，projection 同时呈现当前 DAG。
- Falsifiable predictions:
  - If true: 同一 request 可见 control history 中的 `map_state` 和唯一 active projection；result bytes 随控制次数累积。
  - If false: control result 不含完整状态，或旧 result 不进入后续 provider history。
- Diagnostic evidence plan:
  - Prediction or clause under test: provider-visible Map 当前状态是否存在两个事实载体。
  - Signal: control output schema/bytes、projection count、message LCP。
  - Capture method: 解析 canonical task context events 和 exact payload scan，补 section observer fixture。
  - Event name or marker:
    - `TaskSpaceControlResultR6V1`
    - `TaskSpaceMapEpochSnapshotR6V1`
  - Correlation keys:
    - call_id、map_id、revision、request_id
  - Differentiates from:
    - active projection 自身累加
  - Supports if:
    - result 含 `map_state`，后续请求 projection count=1，且 result message 位于 LCP history。
  - Refutes if:
    - result 只有 delta/ref 或 result 未进入 provider history。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - promote after repair
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: confirmed；六个 run projection marker count 恒为 1，但 31 个 control output 中 30 个携带较新完整 `map_state`，并保留在自然历史；两者可能冲突而非只是等价重复。
- Repair design readiness: ready
- Next step: F1 将当前完整 Map 收敛到 projection，result 保留 revision/delta/error/ref。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: named/auto/named 是 uncached input 的主要放大器
- Status: confirmed
- Parent: P-001
- Claim: TaskSpace bootstrap/work/terminal 的 tool_choice 和 tools shape 切换破坏严格前缀，最终长上下文 named request 形成主要 uncached 增量。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - cache trace 显示每个 R6 run 有两次 choice/shape transition，Standard 为零。
- Falsifiable predictions:
  - If true: 最终 named request 的 first_diff_path 为 tools/tool_choice，prefix_preserved=false，且贡献大部分 R6-vs-Standard uncached delta。
  - If false: 最终 request shape 与前一请求相同，或其 uncached input 很低。
- Diagnostic evidence plan:
  - Prediction or clause under test: terminal shape transition 与 uncached input 的共现和量级。
  - Signal: tool_choice kind、tools hash、first diff、uncached input。
  - Capture method: 聚合六个 provider cache trace 的首尾 named request。
  - Event name or marker:
    - `TaskSpaceProviderCacheTraceV3`
  - Correlation keys:
    - request_id、previous_request_id、epoch_id
  - Differentiates from:
    - provider 首请求冷启动
  - Supports if:
    - 每 run terminal transition 存在，且最终 named requests 的 uncached 总量接近或超过 aggregate delta 的 50%。
  - Refutes if:
    - shape 未改变或 terminal uncached 低于 aggregate delta 的 20%。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - retain as permanent observability
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: confirmed；每个 run 两次 prefix break，terminal first diff 均为 `tools`，最终 named requests 的 uncached 总量为 simple 11,262、complex 26,398。
- Repair design readiness: ready
- Next step: F2 验证 immutable schema 与稳定 `required` choice。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-004: 唯一 projection marker 仍可能是旧快照
- Status: confirmed
- Parent: P-001
- Claim: steady-state provider composer 只保留首次 projection，没有按 canonical DAG 当前 revision/hash 刷新，因此唯一 projection 可能落后于 control result 和真实 Map。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - provider composition 检查历史是否存在 marker，而 exact scanner 只验证 marker 数量和区块形状。
- Falsifiable predictions:
  - If true: 后续请求中的 projection message hash 与 bootstrap 完全相同，而 control result revision 已持续增加。
  - If false: 每次状态提交后 projection identity/revision/hash 随 canonical DAG 更新。
- Diagnostic evidence plan:
  - Prediction or clause under test: projection 唯一性是否同时满足 freshness。
  - Signal: provider message hash/bytes、bootstrap projection 内容、control committed revision、steady-state composer code path。
  - Capture method: 对 Phase E provider trace 与 canonical task context 做 revision/hash 对账，并检查 provider composer。
  - Event name or marker:
    - `TaskSpaceMapEpochSnapshotR6V1`
    - `TaskSpaceControlResultR6V1`
  - Correlation keys:
    - request index、map id、revision、projection hash
  - Differentiates from:
    - H-002 的等价状态重复
  - Supports if:
    - projection message 保持 bootstrap identity，同时 control committed revision 大于 bootstrap。
  - Refutes if:
    - projection revision/hash 与每次 canonical state 一致。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - promote after repair
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: confirmed；六个 run 的每个请求都保留同一个 139-byte bootstrap projection hash，而 control revision 已推进到 4/6。
- Repair design readiness: ready
- Next step: F0 增加 projection identity，F1 改为 ephemeral current projection composer。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: Request/input 固定公式分解
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: Phase E token summary and provider cache trace
- Prediction or plan link:
  - H-001 diagnostic evidence plan
- Matched signal:
  - 新增 request 在 Standard 平均 request 成本下解释 input delta 的比例高于 25%。
- Correlation keys:
  - Phase E run roots
- Raw content:
  ```text
  simple std_req=20 std_input=140021 r6_req=28 r6_input=213502
  request_delta_contribution=76.22160830690927%
  complex std_req=37 std_input=396993 r6_req=43 r6_input=536250
  request_delta_contribution=46.229089556175445%
  ```
- Interpretation: 新增 request 是 simple 的主要 input 放大器，也是 complex 的显著组成部分；complex 仍有较大的 per-request/outlier 成本。
- Time: 2026-07-16 19:00

## Evidence E-002: Control 状态重复与 projection 唯一性审计
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: Phase E canonical task context and exact payload scan
- Prediction or plan link:
  - H-002 diagnostic evidence plan
- Matched signal:
  - 每个 exact payload scan 的 active projection count=1，同时成功 control result 含 `map_state`。
- Correlation keys:
  - call_id/map_id/revision/request_id
- Raw content:
  ```text
  simple controls: 14 calls / 14 outputs / 12,731 combined bytes
  complex controls: 17 calls / 17 outputs / 17,036 combined bytes
  outputs_with_map_state: 30/31
  active_projection_count: 1 in every scanned provider payload
  ```
- Interpretation: projection 没有累加；重复来自历史 control `map_state` 与当前 projection 并存。
- Time: 2026-07-16 19:00

## Evidence E-003: Terminal cache-shape 审计
- Related hypotheses:
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: Phase E provider cache trace
- Prediction or plan link:
  - H-003 diagnostic evidence plan
- Matched signal:
  - 六个 run 均有两次 choice/prefix break，terminal first diff 为 tools，最终 request uncached 较高。
- Correlation keys:
  - request_id/previous_request_id/epoch_id
- Raw content:
  ```text
  simple: choice_changes=2/run, prefix_breaks=2/run,
          terminal_uncached=3783+3836+3643=11262, first_diff=tools
  complex: choice_changes=2/run, prefix_breaks=2/run,
           terminal_uncached=7464+12287+6647=26398, first_diff=tools
  aggregate R6-vs-Standard uncached delta: simple=14601, complex=26489
  ```
- Interpretation: terminal shape switch 是最明确的 uncached 放大器；最终 request 仍有自然动态尾部，因此该数值不是纯因果反事实。
- Time: 2026-07-16 19:00

## Evidence E-004: Projection freshness 与 control revision 对账
- Related hypotheses:
  - H-004
- Direction: supports
- Type: diagnostic-log
- Source: Phase E provider cache trace、canonical task context、provider composer
- Prediction or plan link:
  - H-004 diagnostic evidence plan
- Matched signal:
  - 所有后续请求的 projection message 与首次 bootstrap hash 相同，而 control revision 持续增加。
- Correlation keys:
  - request index/map id/revision/projection hash
- Raw content:
  ```text
  bootstrap projection:
    TaskSpaceMapEpochSnapshotR6V1:
    - map: none
    - bootstrap_required: true

  six runs:
    projection message count = provider request count
    unique projection message hash count = 1
    projection message sha256 = 2536ee26eed4e3dffe372a3b8b5a4c4a6c2e6a8177890a01836e1a6116d97a75
    control revision ranges = 2..6, 2..6, 2..4, 2..6, 2..6, 2..6

  steady-state code:
    clone_history().raw_items().any(is_action_map_epoch_snapshot_developer_item)
    if !should_inject_full_context && !action_map_epoch_snapshot_present { build_developer_context() }
    prepare_provider_visible_prompt_items(items) { items }
  ```
- Interpretation: scanner 的 uniqueness/replacement_confirmed 是 false positive；Agent 实际依赖较新的 control `map_state` 弥补 stale projection。
- Time: 2026-07-16 19:15
