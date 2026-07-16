# Problem P-001: R6 TaskSpace 上下文与缓存成本高于 Standard
- Status: open
- Created: 2026-07-16 18:52
- Updated: 2026-07-17
- Objective: 定位并消除不必要的请求重复、Map 状态重复和 provider cache shape 变化，同时保持语义透传与 R6 correctness。
- Symptoms:
  - simple R6/Standard request=1.40x、input=1.52x、uncached=3.33x。
  - complex R6/Standard request=1.16x、input=1.35x、uncached=2.33x。
  - Phase F final 相对 Phase E R6 baseline 继续回归：simple request/input/uncached 分别增加
    53.6%/100.6%/218.1%，complex 增加 34.9%/61.1%/111.5%。
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
- Current conclusion: H-001/H-002/H-003/H-004/H-005/H-006 的局部机制已处理，但 Phase F 端到端成本目标
  未达成，不能直接进入 Phase G。E-013 至 E-017 证明 F final 同时增加 provider request 和每 request 固定负载；
  immutable lifecycle schema 在仍保留 named/auto/named choice break 时放大 terminal uncached input。F5.0 同版本
  A/B/C 分别 6/6、5/6、6/6 生成非法 `finish.goal`，已反证 schema breadth/description 显著性归因。
  F5.0b 又证明对象类型不是原因；E 对象命名束 6/6 合法，确认 H-012 identity 命名相似性根因。
  F3 的 bind continuation 被使用，但 complete -> bind 没有自然合并，
  更细 Map 的生命周期继续逐请求推进。F3.5 只修复 F1 自引入的前缀断裂，不是 E 到 F 的净成本收益。
  修复计划已插入 `docs/v0.0.5/build-R6/18-r6-phase-f5-cost-regression-repair-plan.md`；Phase G 在 F5.3
  通过前 blocked。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
  - H-008
  - H-009
  - H-010
  - H-011
  - H-012
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

## Hypothesis H-006: TaskSpace 参数解析未消费完整 JSON 文档
- Status: confirmed
- Parent: P-001
- Claim: `taskspace_control` 的路径感知反序列化只读取第一个 JSON 值，没有检查 deserializer 尾部，因而会静默接受
  多余字符；Event Store、sequence manifest、observer 与实际执行对同一原始 call 得出不同结论。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - F4 complex 第 3 轮同一 `call_id` 的原始参数严格解析报 unmatched `}`，但 control result 提交成功。
- Falsifiable predictions:
  - If true: `serde_path_to_error::deserialize` 可返回合法首值，而后续 `Deserializer::end()` 对同一字符串失败。
  - If false: handler 实际收到经过变换的参数，或当前 parser 已验证完整输入。
- Diagnostic evidence plan:
  - Prediction or clause under test: 原始参数、handler parser 和严格 parser 的消费边界差异。
  - Signal: 同一 call id 的 raw arguments、typed result、严格 parse 结果和 parser source。
  - Capture method: F4 rollout call/result 对账、`jq fromjson` 探针、源码检查。
  - Event name or marker:
    - `taskspace.control_arguments_rejected`
    - `TaskSpaceControlResultR6V1`
  - Correlation keys:
    - `call_00_9bv3ewDTFxfe08o33OV27518`
  - Differentiates from:
    - provider 流拼接修复、observer 合法参数误报、call/output 配对错误
  - Supports if:
    - raw arguments 严格解析失败，result 却成功，且 parser 未调用 `Deserializer::end()`。
  - Refutes if:
    - handler payload 与 Event Store raw arguments 不同，或 result 属于其他 call id。
- Evidence gate: satisfied
- Related evidence:
  - E-010
- Conclusion: confirmed；TaskSpace 路径感知 parser 未检查尾部，Standard 的普通 `from_str` 路径不受影响。
- Repair design readiness: ready
- Next step: repaired and verified；继续由 F4 报告保留 malformed control 计数，不做参数修复或重试。
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

## Evidence E-010: F4 malformed control 被静默执行的 call/result 对账
- Related hypotheses:
  - H-006
- Direction: supports
- Type: runtime-trace
- Source: F4 complex pair-003 TaskSpace rollout、严格 JSON 探针、control parser source
- Prediction or plan link:
  - H-006 diagnostic evidence plan
- Matched signal:
  - 原始 bind+patch 参数末尾多一个 `}`，严格解析失败；相同 call id 的 result 却为 committed revision 4。
  - `deserialize_arguments` 调用 `serde_path_to_error::deserialize` 后直接返回，没有调用 `deserializer.end()`。
- Correlation keys:
  - run `target/r6-phase-f4/subscription-billing-repair/20260716-231537-674`
  - call `call_00_9bv3ewDTFxfe08o33OV27518`
- Raw content:
  ```text
  jq fromjson: Unmatched '}' at line 1, column 1726
  call output: status=committed success=true committed_revision=4
  source: taskspace_control_args_wire.rs deserialize_arguments() does not call Deserializer::end()
  sequence/observer: strict parse -> unparseable/unknown
  ```
- Interpretation: Runtime 没有重写字段语义，但静默忽略了 malformed 尾部；这违反原始动作保真和确定性 replay，必须在 F4 收口。
- Time: 2026-07-16 23:30

## Evidence E-011: 完整 JSON 消费修复与复杂样本回归
- Related hypotheses:
  - H-006
- Direction: supports
- Type: fix-validation
- Source: control args fixture、control/sequence 回归、复杂 TaskSpace-only Docker sample
- Prediction or plan link:
  - H-006 repair validation
- Matched signal:
  - 合法首值后追加 `}` 或第二个 JSON 值均返回 typed `protocol_failed`、`state_commit=false`、`partial_commit=0`。
  - control 25/25、sequence 13/13、call identity 1/1 通过。
  - 复杂 live 样本 solved，Map 闭合，无 orphan/parse error；该轮没有自然产生 malformed 参数，因此只作为无回归证据。
- Correlation keys:
  - run `target/r6-phase-f4-strict-parser/subscription-billing-repair/20260716-233221-635`
- Raw content:
  ```text
  rejects_trailing_json_instead_of_executing_the_first_value: passed
  taskspace_control: 25 passed
  tools::sequence::tests: 13 passed
  malformed_tool_arguments_preserve_call_identity_in_feedback: passed
  live: solved, requests=14, nodes=5, edges=5, open=0, orphan=0, parse_errors=0
  cache request2+=85.23%, prefix=11/13
  ```
- Interpretation: TaskSpace 与 Standard 都只接受一个完整 JSON 文档；Runtime 不再静默修复 Agent 尾部错误，反馈身份和零提交语义保持一致。
- Time: 2026-07-16 23:35

## Evidence E-012: 最终 complex 三次矩阵自然验证 malformed 零提交
- Related hypotheses:
  - H-006
- Direction: supports
- Type: fix-validation
- Source: final complex pair-002 TaskSpace rollout
- Prediction or plan link:
  - H-006 live fix validation
- Matched signal:
  - continuation 以错误 `]` 闭合，strict parser 和 observer 均判定 parse error。
  - 同 call id 返回 `protocol_failed`、`state_commit=false`、`partial_commit=0`，没有执行嵌套 patch。
  - Agent 随后以新 bind call 推进，最终 revision 8 terminal commit；public/hidden validator 通过。
- Correlation keys:
  - run `target/r6-phase-f4-final/subscription-billing-repair/20260716-233621-580`
  - call `call_00_j0v0tmnubWUADxcvKbk77016`
- Raw content:
  ```text
  strict parse: Unmatched ']' at line 1, column 1854
  output: status=protocol_failed success=false state_commit=false partial_commit=0
  final matrix: Standard 3/3 solved; R6 3/3 solved; R6 open nodes=0
  ```
- Interpretation: 执行、Event Store、observer 和 Agent feedback 对 malformed action 的语义现已完全一致。
- Time: 2026-07-16 23:45

## Hypothesis H-007: Immutable lifecycle schema 在残留 choice break 下同时放大固定 input 与 terminal uncached input
- Status: confirmed
- Parent: P-001
- Claim: F2/F3 将 bootstrap/work/terminal 的阶段化工具面替换为每 request 固定的完整 13-tool lifecycle schema；
  但 DeepSeek thinking 模式不接受 generic `required` choice，named/auto/named 仍然存在。因此 schema 没有消除
  cache break，反而让每个普通请求和最终 named request 都携带完整工具合同。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-003
- Falsifiable predictions:
  - If true: F final tools section 每 request 大于 Phase E/F1 阶段化 tools 平均值；terminal choice break 的
    uncached input 显著高于 Phase E。
  - If false: 完整 schema 不增加 tools bytes，或 terminal 请求保持上一请求 cache prefix。
- Diagnostic evidence plan:
  - Signal: 各阶段 tools bytes/request、terminal tool choice、terminal uncached input。
  - Capture method: F1/F4 section trace、E/F terminal provider cache trace、tool visibility diff。
  - Differentiates from: natural history 增长和 projection 体积。
- Evidence gate: satisfied
- Related evidence:
  - E-013
  - E-014
- Conclusion: confirmed；完整 schema 为 26,628 B/request，阶段化 bootstrap/work/terminal 分别为
  15,999/20,513/3,541 B。terminal 仍发生 choice break，F simple/complex terminal uncached 总量分别比 E
  增加 144.5%/68.1%。
- Repair design readiness: ready
- Next step: 按 Phase F5.1 恢复 hard-state 对齐工具面并执行独立收益门。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-008: 合并后的完整 control schema 降低 bootstrap 合同显著性并制造稳定初始化重试
- Status: refuted
- Parent: P-001
- Claim: F2 将 bootstrap-only control 描述与 schema 合并进七类 lifecycle `anyOf`，同时把顶层描述从明确的
  `node_id-only Finish identity` 改为通用 lifecycle 描述；DeepSeek 在 `strict=false` 下稳定给 Finish 添加 goal，
  每次失败都产生下一次 provider request。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Falsifiable predictions:
  - If true: E 阶段化 bootstrap 的首次初始化非法 `finish.goal` 接近 0，而 F final 在相同两个 sample 中稳定发生；
    只恢复 bootstrap schema 显著性后该错误应下降。
  - If false: 同版本 bootstrap-only 或显式 Finish 描述不能降低 `finish.goal`。
- Diagnostic evidence plan:
  - Signal: 首次 initialize 参数、错误路径、bootstrap tool description/schema shape。
  - Capture method: E/F 各六次 rollout 对账与提交 diff；最终因果门需要同版本 schema A/B probe。
  - Differentiates from: Agent 对 Root/Finish 产品概念本身不理解。
- Evidence gate: satisfied
- Related evidence:
  - E-015
  - E-018
- Conclusion: refuted。A full、B bootstrap-only generic、C bootstrap-only explicit 的正式结果为
  `finish.goal=6/6、5/6、6/6`，且唯一字段错误路径相同。schema breadth 与 description salience 都不是
  初始化回归的原因；前一轮有效重复观察为 6/6、6/6、6/6。
- Repair design readiness: not_applicable
- Next step: 不实施 H-008 修复；由 H-011 继续隔离 Finish identity wire shape。
- Blocker:
  - none
- Close reason: falsified by same-version provider A/B/C

## Hypothesis H-011: Finish identity 的对象线形态诱发 `goal` 泛化
- Status: refuted
- Parent: P-001
- Claim: `initialize_map.finish` 以对象形式与 Root/Work 节点并列，虽然 schema 只允许 `node_id` 且已有明确描述，
  DeepSeek 在 `strict=false` 下仍按普通节点形态补齐 `goal`。
- Layer: root-cause
- Factor relation: alternative_to
- Depends on:
  - none
- Falsifiable predictions:
  - If true: 当前 Finish 对象高复现错误；仅改变 identity wire shape 后错误降到 <=1/6，且其他字段不回归。
  - If false: 对象命名或标量 identity 仍产生非法字段/类型错误，或错误来自实验 prompt/history。
- Diagnostic evidence plan:
  - Signal: Finish 字段类型、键集合、parse verdict、其他 initialize 字段错误路径。
  - Capture method: F5.0b current object / distinct object / scalar identity 三臂 provider probe。
  - Differentiates from: schema breadth、顶层 description、Runtime reject、projection/history。
- Evidence gate: satisfied
- Related evidence:
  - E-018
  - E-019
- Conclusion: refuted。E 仍使用对象，但 6/6 严格合法；F 标量反而有 1/6 生成空对象。对象类型不是必要根因。
- Repair design readiness: not_applicable
- Next step: 不实施标量化；由 H-012 承接获胜的命名束归因。
- Blocker:
  - none
- Close reason: named object arm eliminated the error without scalar conversion

## Hypothesis H-012: Finish identity 与普通节点共享命名束诱发字段泛化
- Status: confirmed
- Parent: P-001
- Claim: 当前 `finish: { node_id }` 同时复用普通节点的外层语义词和内部 `node_id` 词形，DeepSeek 在
  `strict=false` 下按普通节点补齐 `goal`；使用独立 identity 命名束可保持对象语义并消除泛化。
- Layer: root-cause
- Factor relation: alternative_to
- Depends on:
  - none
- Falsifiable predictions:
  - If true: D 当前命名高复现 `finish.goal`，E=`finish_identity: { id }` 降到 <=1/6，公共字段不回归。
  - If false: E 仍高复现 identity 错误，或只有标量 F 能通过。
- Diagnostic evidence plan:
  - Signal: identity error、common graph error、actual schema shape、request input/cache。
  - Capture method: F5.0b D/E/F provider probe。
  - Differentiates from: 对象类型、schema breadth、description、Runtime reject、projection/history。
- Evidence gate: satisfied
- Related evidence:
  - E-019
- Conclusion: confirmed。D identity error=5/6，E=0/6，F=1/6；E 公共字段错误=0，且 schema 只比 D 增加 8 bytes。
- Repair design readiness: ready
- Next step: F5.0c 一次性切换 E wire contract，不保留旧字段兼容。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-009: F3 没有承载 complete -> bind 边界，细粒度 Map 继续逐请求推进
- Status: confirmed
- Parent: P-001
- Claim: F3 只允许 initialize/bind/mutate 携带 continuation，明确禁止 complete continuation，并把
  `complete -> bind -> action` 寄托于同一 provider response 的 sibling calls；正式运行中 Agent 没有生成任何
  multi-control response，因此每个 Work 边界仍需要额外 provider request。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Falsifiable predictions:
  - If true: bind continuation 被采用，但 multi-control carrier 为 0，且每 run 存在多个无 follow-up 的
    nonterminal transition。
  - If false: complete 与下一 bind 在同一 response 中稳定出现，或 transition 不形成独立请求。
- Diagnostic evidence plan:
  - Signal: bind continuation、multi-control carrier、nonterminal transition without follow-up、Map Work 数。
  - Capture method: F4 performance observation 与原始 rollout cadence。
  - Differentiates from: control reject 产生的重试。
- Evidence gate: satisfied
- Related evidence:
  - E-016
- Conclusion: confirmed；6 个 F4 run 均使用 bind continuation，但 multi-control carrier 全为 0，每 run 仍有
  3-4 个 nonterminal transition 没有 sibling follow-up。F final 每张 Map 固定 3 个 Work，E 为 1-2 个 Work；
  额外 Work 本身未证明不合理，但现有 carrier 让它的机械状态迁移直接转化为 request 成本。
- Repair design readiness: ready
- Next step: 按 Phase F5.2 在现有 control tool 内恢复 Agent 声明的 complete handoff carrier。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-010: Phase F 验收合同允许局部机制完成但端到端成本回归
- Status: confirmed
- Parent: P-001
- Claim: F0-F4 的退出门分别验证 section 观测、反馈去重、schema hash、continuation 能力和 prefix 恢复，
  但没有要求 final request/input/weighted input 不劣于 Phase E；F3 还允许“Agent 未采用时只确认机制”，F4 明确
  性能只报告不设收益门。因此局部结果可以全部 PASS，同时总体成本翻倍。
- Layer: process-root-cause
- Factor relation: enables
- Depends on:
  - none
- Falsifiable predictions:
  - If true: 计划门禁不存在 E->F 端到端成本阈值，最终报告仍能在成本显著回归时写成 6/6 complete。
  - If false: 任一正式 gate 会因 F final 成本高于 E 而阻止 Phase F 完成。
- Diagnostic evidence plan:
  - Signal: Phase F 计划与结果的总退出门和状态。
  - Capture method: 文档合同审计。
- Evidence gate: satisfied
- Related evidence:
  - E-017
- Conclusion: confirmed；这是回归被接受的流程根因，不是 provider token 增长的运行时机制。
- Repair design readiness: ready
- Next step: Phase F 已重开；F5.3 以 Phase E requests/input/uncached/weighted input 为硬 outcome gate。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-013: E 到 F 的 request 数与平均 request 负载双重分解
- Related hypotheses:
  - H-007
  - H-009
- Direction: supports
- Type: diagnostic-log
- Source: Phase E E6 与 Phase F4 performance observation
- Prediction or plan link:
  - E->F regression attribution
- Matched signal:
  - simple/complex 都同时增加 request count 和平均 input/request，不是单一 outlier 或单一 cache 指标。
- Raw content:
  ```text
  simple E: req=28 input=213502 avg=7625.1
  simple F: req=43 input=428351 avg=9961.7
  delta=214849; request-count effect=114376 (53.2%); per-request effect=100473 (46.8%)

  complex E: req=43 input=536250 avg=12470.9
  complex F: req=58 input=863940 avg=14895.5
  delta=327690; request-count effect=187064 (57.1%); per-request effect=140626 (42.9%)
  ```
- Interpretation: 根因必须同时解释约 55% 的请求数量效应和约 45% 的单请求负载效应。
- Time: 2026-07-17 16:10

## Evidence E-014: 阶段化工具面与完整 lifecycle schema 的 wire 体积和 terminal break
- Related hypotheses:
  - H-007
- Direction: supports
- Type: runtime-trace
- Source: F1/F3/F4 provider section trace 与 E/F terminal provider cache trace
- Prediction or plan link:
  - H-007 diagnostic evidence plan
- Matched signal:
  - F1 阶段化工具面按状态缩小；F3 后完整 schema 固定为 26,628 B，tool choice 仍在 terminal 变回 named。
- Raw content:
  ```text
  phase-scoped tools: bootstrap=15999 B, work=20513 B, terminal=3541 B
  F2 deduplicated lifecycle schema=24449 B/request
  F3/F4 lifecycle + continuation schema=26628 B/request
  Standard tools=21669 B/request

  terminal uncached simple E=11262, F=27532 (+144.5%)
  terminal uncached complex E=26398, F=44368 (+68.1%)
  F terminal tools section=26628 B and cached_input=0 in all six runs
  ```
- Interpretation: immutable schema 只稳定了 tools hash，没有稳定 tool choice；在 terminal break 上携带完整 schema
  明确放大未缓存输入。F3 continuation 又在 F2 去重后增加 2,179 B/request。
- Time: 2026-07-17 16:15

## Evidence E-015: E/F 首次初始化参数与重试对账
- Related hypotheses:
  - H-008
- Direction: supports
- Type: runtime-trace
- Source: Phase E E6 与 Phase F4 simple/complex 各三次 rollout
- Prediction or plan link:
  - H-008 diagnostic evidence plan
- Matched signal:
  - E 六次首次 initialize 均合法；F 六次第一次都生成禁止的 `finish.goal`。
- Raw content:
  ```text
  Phase E initialize_map attempts: 1/1/1 simple, 1/1/1 complex
  Phase F initialize_map attempts: 2/2/4 simple, 2/2/2 complex
  F first-attempt finish.goal violations: 6/6
  additional F initialization requests: simple=5, complex=3
  one simple run additionally over-corrected by deleting required root.goal
  ```
- Interpretation: 初始化重试是稳定回归，不是 F4 pair-002 单一 outlier。当前证据将回归窗口收敛到完整 schema/描述
  切换，但在 repair 前仍需同版本 A/B 隔离模型随机性。
- Time: 2026-07-17 16:20

## Evidence E-016: Map 粒度、control cadence 与未发生的 sibling batching
- Related hypotheses:
  - H-009
- Direction: supports
- Type: runtime-trace
- Source: Phase E/F performance observation 与 F4 rollout cadence
- Prediction or plan link:
  - H-009 diagnostic evidence plan
- Matched signal:
  - F final Map 的 Work 数和 control 数稳定高于 E；F3 continuation 只合并 bind 与 ordinary action，未合并节点边界。
- Raw content:
  ```text
  Phase E nodes simple=4/4/3, complex=4/4/4
  Phase F nodes simple=5/5/5, complex=5/5/5
  Phase E control total simple=14, complex=17
  Phase F control total simple=30, complex=26
  F bind continuation: naturally adopted in every run
  F multi_control_carrier_responses=0/6 runs
  F nonterminal_transitions_without_follow_up=3/4/4 simple, 3/4/3 complex
  ```
- Interpretation: 三 Work 的 Map 对用户任务并非明显错误，不能通过 Runtime 强迫合并；问题是现有 tool carrier 没有
  把相邻机械状态迁移承载在同一 Agent 声明中。
- Time: 2026-07-17 16:25

## Evidence E-017: 最终 section 构成与排除项
- Related hypotheses:
  - H-007
  - H-009
  - H-010
- Direction: supports
- Type: diagnostic-log
- Source: F4 section report、projection identity trace、Phase F plan/result
- Prediction or plan link:
  - final payload attribution and gate audit
- Matched signal:
  - tools 是最大固定段；自然历史和普通工具反馈随新增请求增长。active projection 只占 2-3%，epoch hash/expected
    identity 属于诊断日志，不进入 provider prompt。
- Raw content:
  ```text
  F simple estimated section tokens:
    tools=286251, natural_history=82920, ordinary_feedback=49792,
    control_feedback=23921, projection=10855, system=7740
  F complex:
    tools=386106, natural_history=226748, ordinary_feedback=196203,
    control_feedback=33869, projection=19660, system=10440

  projection count=1/request; semantic replacement=0
  F3.5 fixed F1 prefix 0% to final 85.00%/89.09%
  Phase F final gate: performance only reports observed values
  ```
- Interpretation: F1 的 map_state 去重是净收益，F3.5 epoch 是必要的自回归修复；二者不是 final 成本主因。
  不能把诊断 identity 字段误算为 provider-visible input，也没有证据支持通过语义裁剪解决当前回归。
- Time: 2026-07-17 16:30

## Evidence E-018: F5.0 同版本 bootstrap 三臂因果实验
- Related hypotheses:
  - H-008
  - H-011
- Direction: refutes H-008; motivates H-011
- Type: provider-experiment
- Source: 生产 schema builder 导出器、三臂 DeepSeek probe、脱敏参数结构日志
- Prediction or plan link:
  - Phase F5.0 H-008 evidence gate
- Matched signal:
  - A full lifecycle、B bootstrap-only generic、C bootstrap-only explicit 各执行 simple/complex 3 次。
  - 18/18 HTTP 200、18/18 单 `taskspace_control`、18/18 参数可解析。
  - A/B/C 为 `finish.goal=6/6、5/6、6/6`；17 次失败的唯一字段错误均为 `unexpected:finish.goal`。
  - B/C schema input 明显低于 A，但正确性没有改善。
- Raw content:
  ```text
  artifact: target/r6-f5-bootstrap-ab/20260717-live-03/provider-capability.json
  events: target/r6-f5-bootstrap-ab/20260717-live-03/probe-events.jsonl
  schema bytes A/B/C: 9427 / 4406 / 4041
  input total A/B/C: 17208 / 8622 / 8226
  finish.goal A/B/C: 6/6 / 5/6 / 6/6
  request2+ cache A/B/C: 98.19% / 97.98% / 93.36%
  ```
- Interpretation: H-008 的两个候选因子都被同版本实验反证。不能因为 bootstrap-only 更省 token 就把它当作
  初始化修复。Finish 对象线形态是下一假设，但在 F5.0b 前不得实现 Runtime 纠错、projection 提示或生产字段改动。
- Time: 2026-07-17 19:20

## Evidence E-019: F5.0b Finish identity 命名束与类型三臂实验
- Related hypotheses:
  - H-011
  - H-012
- Direction: refutes H-011; supports H-012
- Type: provider-experiment
- Source: 生产 bootstrap schema 派生器、D/E/F DeepSeek probe、严格公共字段校验
- Prediction or plan link:
  - Phase F5.0b Finish identity evidence gate
- Matched signal:
  - D 当前对象、E 独立命名对象、F 独立命名标量各执行 simple/complex 3 次。
  - 18/18 HTTP 200、18/18 单 control call、18/18 参数可解析，公共图字段错误为 0。
  - D identity error=5/6，均为 `unexpected:finish.goal`；E=0/6；F=1/6，错误为标量位置生成空对象。
  - E 与 D 都是对象，证明对象类型不是根因；E 只比 D 增加 8 schema bytes。
- Raw content:
  ```text
  artifact: target/r6-f5-finish-identity-ab/20260717-live-01/provider-capability.json
  analysis: target/r6-f5-finish-identity-ab/20260717-live-01/analysis.json
  schema bytes D/E/F: 4406 / 4414 / 4247
  valid D/E/F: 1/6 / 6/6 / 5/6
  identity errors D/E/F: 5/6 / 0/6 / 1/6
  common errors D/E/F: 0/6 / 0/6 / 0/6
  input total D/E/F: 8622 / 8634 / 8388
  request2+ cache D/E/F: 97.98% / 97.85% / 91.56%
  ```
- Interpretation: 生产候选冻结为 E=`finish_identity: { id }`。不能选择更小但有类型错误的 F，也不需要增加
  Runtime 修复、projection 提示或 reasoning 解析。
- Time: 2026-07-17
