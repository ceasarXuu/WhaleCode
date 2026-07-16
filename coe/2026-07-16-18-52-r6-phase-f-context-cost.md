# Problem P-001: R6 TaskSpace 上下文与缓存成本高于 Standard
- Status: open
- Created: 2026-07-16 18:52
- Updated: 2026-07-16 22:25
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
- Current conclusion: H-001/H-002/H-003/H-004/H-005 均已确认；F1/F2/F3 已完成状态去重、稳定 schema
  和 Agent 声明动作合并，F3.5 正在把错误的逐请求 current projection 替换为固定 epoch baseline + canonical
  delta journal，以恢复 provider 严格前缀。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
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
- Next step: repaired and verified in F1；继续用 F4 多次矩阵观察随机生命周期失败。
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

## Hypothesis H-005: 逐请求尾部替换 projection 结构性破坏 provider 前缀
- Status: confirmed
- Parent: P-001
- Claim: F1 的 ephemeral current projection 每轮从历史过滤并重新追加到尾部，使下一轮新增自然历史插入到上一轮
  projection 的位置，导致同一 tool schema 下也无法保持消息前缀。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-004
- Rationale:
  - 当前 composer 固定执行 `filter(old projection) -> push(current projection)`；严格前缀缓存要求上一请求完整消息序列是
    下一请求的前缀，两者在第一个动态 response 后即不可能同时成立。
- Falsifiable predictions:
  - If true: TaskSpace 后续请求的 first diff 位于上一轮 projection 所在 message index，message prefix
    preservation 接近 0%，即使 tools hash 唯一且 projection revision 未变化。
  - If false: 同一 revision 的连续请求保持上一请求完整 message prefix，request 2+ cache hit 接近 Standard。
- Diagnostic evidence plan:
  - Prediction or clause under test: projection 尾部替换与 prefix/cache 的结构性关系。
  - Signal: message shapes/LCP、projection index/hash/revision、tools hash、request 2+ cache hit。
  - Capture method: 对 F3 simple/complex provider wire trace 逐 request 对账，并检查 composer 顺序。
  - Event name or marker:
    - `TaskSpaceProviderCacheTraceV3`
    - `TaskSpaceMapEpochSnapshotR6V1`
  - Correlation keys:
    - epoch_id、request_id、previous_request_id、projection_sha256
  - Differentiates from:
    - provider 冷启动、tool schema 变化和 projection 体积过大
  - Supports if:
    - 两个不同复杂度 sample 的 TaskSpace prefix preservation 均接近 0%，且 tools hash 恒定。
  - Refutes if:
    - projection 尾部替换下仍保持高消息前缀，或低命中只发生在首请求。
- Evidence gate: satisfied
- Related evidence:
  - E-009
- Conclusion: confirmed；simple/complex 分别为 0/12、0/18 message prefix，request 2+ cache hit
  13.22%/13.27%，而 Standard 为 94.85%/92.34%。
- Repair design readiness: ready
- Next step: F3.5 固定 epoch baseline 锚点，后续只追加原始 canonical delta journal。
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

## Evidence E-005: F0 provider section 与 projection identity 观测闭环
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Direction: supports
- Type: diagnostic-log
- Source: `provider_wire_trace.rs`、provider section fixtures、benchmark observer self-tests
- Prediction or plan link:
  - Phase F0 exit gate
- Matched signal:
  - 八类 section bytes 之和与最终 provider payload bytes 精确相等。
  - Standard 的 TaskSpace-only section 为零；缺失 projection identity 显式标记 unavailable。
  - TaskSpace active projection 暴露 map hash、revision 和 projection hash，不保存原始内容。
  - 聚合报告保留逐 request 样本，并输出总和、均值和中位数。
- Raw content:
  ```text
  cargo test -p codex-core provider_wire_trace --lib -- --nocapture: 11 passed
  test-cost-instrumentation.ps1: passed
  test-performance-observation.ps1: passed
  test-harness.ps1: passed
  implementation commit: 12b479171
  ```
- Interpretation: F0 已提供验证 F1 ownership/freshness 改造所需的无原文机械观测基线。
- Time: 2026-07-16 19:35

## Evidence E-006: F1 projection freshness 与当前 Map 单 owner 闭环
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: F1 deterministic tests、simple/complex Docker pair、provider wire trace v3
- Prediction or plan link:
  - Phase F1 exit gate
- Matched signal:
  - simple 15/15、complex 13/13 provider requests 的 projection freshness 全部确认，active section 恒为 1。
  - control success/failure 不再包含 `map_state`；success delta 只引用 canonical domain events。
  - nested ordinary tool call/output 保持原始独立载体，outer control result 不再复制其身份和 event refs。
  - 初始化 feedback 由 E6 1,018 bytes 降到 539 bytes，下降 47.1%。
  - simple/complex Standard 与 R6 均通过 public/hidden validator，R6 Map 全部闭合。
- Raw content:
  ```text
  simple: target/r6-phase-f1-compact-v2/single-file-fast-fix/20260716-205728-404
  complex: target/r6-phase-f1-compact-v2/subscription-billing-repair/20260716-205911-381
  action_map: 67 passed; session: 182 passed; control: 21 passed; sequence: 11 passed
  ```
- Interpretation: H-002 的双 owner 根因已修复；缓存命中差异和状态动作错误仍存在，分别进入 F2/F3，不由 projection 增加语义引导补救。
- Time: 2026-07-16 21:05

## Evidence E-007: F2 immutable tool schema 与 required HOLD
- Related hypotheses:
  - H-003
- Direction: supports
- Type: experiment
- Source: provider capability probe、simple/complex Docker pair、provider cache trace v3
- Prediction or plan link:
  - Phase F2 exit gate
- Matched signal:
  - simple/complex TaskSpace 每个 run 的 tools count 恒为 13，`tools_hash` 唯一值均为 1。
  - 去除 nested/top-level 参数 schema 重复后，tools section 从 35,648 降至 24,449 bytes/request。
  - `required + thinking disabled` 返回预期 tool call；`required + thinking enabled` 返回 HTTP 400，错误为
    `thinking_tool_choice_incompatible`，无 reasoning content。
  - simple/complex 双侧 correctness 通过，R6 Map 均闭合。
- Raw content:
  ```text
  probe: target/r6-phase-f2-provider/probe-required-thinking.json
  simple: target/r6-phase-f2-dedup/single-file-fast-fix/20260716-212621-464
  complex: target/r6-phase-f2-dedup/subscription-billing-repair/20260716-212759-568
  ```
- Interpretation: schema/list 变化根因已消除；provider 不允许 required 保留 thinking，因此 choice 统一不得实施，明确 HOLD。动态 projection/history 仍造成低缓存命中，留待 F4 汇总，不用 Runtime 语义约束补救。
- Time: 2026-07-16 21:35

## Evidence E-008: F3 声明式 continuation 机制与 live adoption
- Related hypotheses:
  - H-001
- Direction: supports
- Type: experiment
- Source: deterministic sequence tests、F3 simple/complex Docker pair、canonical rollout
- Prediction or plan link:
  - Phase F3 exit gate
- Matched signal:
  - bind continuation 在 simple/complex 中分别自然出现 3/2 次；sequence 按声明顺序执行并保持 parent/call identity。
  - 两个 sample 双侧 correctness 通过，R6 Map、Root、Finish 均闭合，无 partial commit。
  - simple R6 request=13，与 F2 基线相同；complex R6 request=19，高于 F2 基线 14，不宣称复杂样本成本收益。
- Raw content:
  ```text
  implementation: 6c7de4cbe
  schema 3/3; args 14/14; sequence 13/13; control 23/23
  simple: target/r6-phase-f3-sequence/single-file-fast-fix/20260716-220547-738
  complex: target/r6-phase-f3-sequence/subscription-billing-repair/20260716-220809-394
  ```
- Interpretation: F3 的工具能力与机械执行正确且被 Agent 使用；复杂样本增加主要来自 Agent 参数/状态纠错，不能由 Runtime
  增加语义约束修补。
- Time: 2026-07-16 22:20

## Evidence E-009: F3 projection 锚点与缓存前缀对账
- Related hypotheses:
  - H-005
- Direction: supports
- Type: diagnostic-log
- Source: F3 provider cache trace v3、performance observation、provider composer source
- Prediction or plan link:
  - Phase F3.5 root-cause gate
- Matched signal:
  - tools hash 在每个 R6 run 内唯一，但 TaskSpace message prefix preservation 为 0；两个复杂度不同的 sample 命中率
    都稳定在约 13%。
  - composer 每轮删除历史 projection 后把 current projection 追加在全部自然历史之后；下一轮增长历史必然占据旧
    projection 的 message index。
- Raw content:
  ```text
  simple: prefix=0/12, request2+ hit=13.22%, cached/total=16,896/127,994
  complex: prefix=0/18, request2+ hit=13.27%, cached/total=34,304/258,860
  standard simple/complex request2+ hit=94.85%/92.34%
  composer: filter(is_projection) -> prepared.push(projection_item)
  ```
- Interpretation: 低缓存是 projection 位置造成的结构性前缀破坏，不应通过裁剪 projection 或增加 Agent 约束解决。
- Time: 2026-07-16 22:25
