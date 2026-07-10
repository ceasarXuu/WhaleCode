# Problem P-001: R5 TaskSpace 在 G1 正确性样本中请求次数显著高于 Standard
- Status: diagnosed
- Created: 2026-07-10 22:56
- Updated: 2026-07-10 22:56
- Objective: 用最终 wire 和原始 tool/control history 精确解释 `count-call-stack` 三轮中 R5 51 requests 对 Standard 22 requests 的29次放大，不把缓存、反馈或 runtime 约束先验地写成根因。
- Symptoms:
  - controlled 3-run 中 Standard 为 8/8/6 requests，R5 为 13/21/17 requests。
  - R5 wall time 为 Standard 的2.26倍，input tokens 为2.41倍。
- Expected behavior:
  - TaskSpace 允许必要的 Map 记账开销，但报告必须区分固定状态工具往返、Agent 额外普通动作、并行承载差异和环境噪音。
  - 不得用缓存失效或反馈丢失解释已经被 G1 证据排除的残余。
- Actual behavior:
  - R5 三轮共51 requests；Standard 共22 requests。
  - 每个非最终 provider response 都产生至少一个工具调用，没有无工具的中间采样循环。
  - R5 比 Standard 多12个状态工具调用和17个普通工具调用；总差29，恰好等于请求差29。
- Impact:
  - correctness 和 cache gate 已通过，但请求、wall time、output 和总 input 成本仍明显放大。
  - 如果把固定 Map 成本、Agent 行为波动和环境问题混成一个原因，后续容易错误增加 runtime 语义约束。
- Reproduction:
  - Run root: `target/r5-g1-repeats/count-call-stack/20260710-210444-351`。
  - 对六个 side 的 `provider-wire-trace.jsonl` 统计 request/tool message 增量。
  - 对三个 R5 `rollout.jsonl` 按 `token_count` 边界重建每个 provider response 的 function calls。
  - 对三个 Standard `whale-exec.jsonl` 和 R5 call/output pair 对账动作内容及失败反馈。
- Environment:
  - Linux/bash，branch `whalecode-alpha`，DeepSeek `deepseek-v4-flash`，G1 append-only history 后样本。
- Known facts:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
  - E-006
- Ruled out:
  - 相邻请求缓存前缀破坏不是本轮请求放大的原因：R5 strict-prefix 48/48，request-2+ cache hit 97.66%。
  - 工具反馈丢失不是本轮主要原因：pre-init hard reject、pytest failure、apply_patch failure 和成功输出都按原文进入后续 history。
  - aggregate 并行工具承载能力下降不是29次放大的原因：Standard/R5 都因并行工具各少发15次请求。
  - 根任务 `active` 或 Result `unreviewed` 没有在本样本制造额外无工具循环：所有中间 response 都对应至少一个工具调用，且没有重复 bind/state_commit。
- Causal boundary:
  - 12个 Map control 是当前三节点 mandatory-map contract 的确定性结构成本，三轮均可复现。
  - 17个额外普通动作是本轮 treatment-correlated observation，不等于都已证明由 TaskSpace 稳定触发。
  - 其中2个 pre-init find/ls 被空 Map 硬规则拒绝，属于明确的 TaskSpace 特有额外动作；第二轮7个 patch/read 循环和其余环境/检查动作仍需更多 controlled repeats 区分上下文影响与模型采样波动。
- Fix criteria:
  - 在用户授权修复后，任何方案必须分别度量固定 control 往返、普通动作数量和并行承载；不得只压低一个总 request 数。
  - 修复验证必须保持 G1 strict-prefix、反馈忠实性、Agent-owned Map 和 correctness。
  - 需要 controlled repeats 证明 request 降低不是模型随机波动。
- Current conclusion: 请求差已被高置信度分解。29次额外请求不是缓存或隐藏 runtime retry，而是工具调用净增29且两侧 aggregate 并行节省量相同：12个为三轮固定的 `initialize_map + 3*finish_node`，17个为 Agent 额外普通动作。17个普通动作进一步由5个额外 inspect/discovery、5个额外 validation/environment probe 和第二轮7个 patch/read 修复循环组成。TaskSpace 的强制 Map 工具协议形成稳定底噪，其中2个 pre-init 拒绝也是明确的 TaskSpace 特有成本；其余15个普通动作只能认定为本轮 Agent 行为差异，尚不能用三轮数据证明是 TaskSpace 的稳定因果效应。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
  - E-006
- Close reason:
  - diagnosis complete; repair not authorized

## Hypothesis H-001: 29次请求差主要由额外工具调用数量直接产生
- Status: confirmed
- Parent: P-001
- Claim: native tool loop 没有生成隐藏的空采样循环；R5 比 Standard 多出的 request 数等于 function/tool calls 的净增量。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - 每次模型返回工具调用后必须接收工具结果并再次采样；如果不存在纯 reasoning 中间轮，request 差应能由工具响应数量解释。
- Falsifiable predictions:
  - If true: Standard 22 requests 应拆成19个工具响应+3个 final；R5 51 requests 应拆成48个工具响应+3个 final。
  - If false: R5 应存在没有 function call 的中间 provider response，或工具响应增量明显小于29。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对最终 wire request 数、tool-role message 增量和 rollout function call 边界进行三轮对账。
  - Signal: request count、每次相邻 request 新增 tool message 数、response function-call list。
  - Capture method: `provider-wire-trace.jsonl` 加 `rollout.jsonl` 独立重建。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
    - `response_item.function_call`
    - `event_msg.token_count`
  - Correlation keys:
    - pair id
    - request index
    - call id
  - Differentiates from:
    - cache miss导致的token放大但不增加请求。
    - runtime内部事件很多但没有触发provider采样。
  - Supports if:
    - 所有非最终 response 均含工具，且额外工具响应数为29。
  - Refutes if:
    - 出现无工具中间 response 或计数无法闭合。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 final-wire/tool cadence 观测
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready, but implementation requires user authorization
- Next step: 将固定 control 与普通动作分账，不立即设计修复。
- Blocker:
  - repair not authorized
- Close reason:
  - not closed

## Hypothesis H-002: R5 请求放大主要因为并行工具承载能力下降
- Status: refuted
- Parent: P-001
- Claim: R5 把原本可在一个 response 中并行的普通工具拆成更多 provider responses，形成主要请求差。
- Layer: sub-cause
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - 单次 R5 trace 中后半段经常每个 response 只有一个工具，表面上像 batching 退化。
- Falsifiable predictions:
  - If true: R5 由并行工具节省的 request 数应显著少于 Standard。
  - If false: 两侧 aggregate 的并行节省量相同，request 差来自工具总量。
- Diagnostic evidence plan:
  - Prediction or clause under test: 比较 `tool call count - tool-bearing response count`。
  - Signal: Standard/R5 三轮 tool message 增量和 response 分组。
  - Capture method: final wire tool-role delta 与 rollout function calls 对账。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
  - Correlation keys:
    - pair id
    - request index
  - Differentiates from:
    - 工具总量增加。
  - Supports if:
    - R5 parallel savings 显著更小。
  - Refutes if:
    - 两侧 savings 相同。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 在 performance observer 后续加入 tool calls/tool-bearing response
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: refuted
- Repair design readiness: not applicable
- Next step: 不把 native parallel batching 作为本轮首要修复方向。
- Blocker:
  - none
- Close reason:
  - aggregate parallel savings are equal

## Hypothesis H-003: 固定 Map 状态工具协议贡献稳定请求底噪
- Status: confirmed
- Parent: P-001
- Claim: Agent 为简单任务创建三个节点后，每轮必须执行一次 `initialize_map` 和三次 `finish_node`，四个纯 control response 都形成下一次 provider sampling，三轮固定贡献12次额外请求。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - Standard 没有 Map 状态工具；R5 controls 不与普通工具并发，且后续动作依赖状态变更结果。
- Falsifiable predictions:
  - If true: 每轮恰好4个 control-only response，无 bind/state_commit 重复。
  - If false: controls 与普通工具同 response 合并，或存在额外 runtime 自动状态动作。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对三轮 taskspace_control call/action 和 request 分组。
  - Signal: `initialize_map=1`、`finish_node=3`、control-only request index。
  - Capture method: rollout response reconstruction 与 `taskspace-control-usage.json` 对账。
  - Event name or marker:
    - `taskspace_control`
  - Correlation keys:
    - map id
    - node id
    - call id
  - Differentiates from:
    - Agent 普通 read/edit/test 动作。
  - Supports if:
    - 三轮合计12 controls 且各自独占工具响应。
  - Refutes if:
    - control 计数或分组不成立。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 control usage 与 request grouping
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: confirmed
- Repair design readiness: ready, but implementation requires user authorization
- Next step: 仅记录这是当前 mandatory-map contract 的结构成本，不让 runtime 自动替 Agent 完成语义状态。
- Blocker:
  - repair not authorized
- Close reason:
  - not closed

## Hypothesis H-004: 剩余普通动作放大来自反馈丢失导致 Agent 重复
- Status: refuted
- Parent: P-001
- Claim: R5 的17个额外普通工具调用主要因为失败或读取结果没有正确进入上下文，Agent 被迫重复。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - 历史 H166 和 cache bug 都曾因工具/状态反馈丢失制造重复动作。
- Falsifiable predictions:
  - If true: 重复前的 hard reject、pytest、patch 或 read 输出应在后续 provider history 中缺失、残缺或被改写。
  - If false: 输出完整可见，后续动作与反馈相符但体现 Agent 自主选择或修正。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查典型重复链的 function_call_output 和 append-only wire。
  - Signal: pre-init reject 原文、pytest stderr、apply_patch failure/success、文件 reread、strict-prefix/cache。
  - Capture method: call/output pair 审计与 G1 cache summary。
  - Event name or marker:
    - `function_call_output`
    - `provider.chat_wire_prefix_preserved`
  - Correlation keys:
    - call id
    - request index
  - Differentiates from:
    - Agent 低质量 patch、环境探测和显式 Map 状态成本。
  - Supports if:
    - 关键反馈缺失或失真。
  - Refutes if:
    - 反馈原文完整且后续 history 严格追加。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 G1 wire 和 tool feedback trace
- Evidence gate: satisfied
- Related evidence:
  - E-004
  - E-005
  - E-006
- Conclusion: refuted
- Repair design readiness: not applicable
- Next step: 不以增加 runtime 语义约束修复普通动作数量。
- Blocker:
  - none
- Close reason:
  - feedback and wire evidence contradict the claim

## Evidence E-001: 三轮请求可完全拆成工具响应和最终回答
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: G1 six-side `provider-wire-trace.jsonl` and R5 `rollout.jsonl`
- Prediction or plan link:
  - H-001 tool-bearing response decomposition
- Matched signal:
  - Standard `22=19 tool responses+3 final`; R5 `51=48 tool responses+3 final`
- Correlation keys:
  - pair-001..003
  - request_index
- Raw content:
  ```text
  Standard requests: 8 + 8 + 6 = 22
  Standard tool-bearing responses: 7 + 7 + 5 = 19
  R5 requests: 13 + 21 + 17 = 51
  R5 tool-bearing responses: 12 + 20 + 16 = 48
  intermediate tool-free responses: 0
  request delta: 29
  ```
- Interpretation: 不存在隐藏的无工具 retry loop；请求数由模型工具响应轮数闭合解释。
- Time: 2026-07-10 22:56

## Evidence E-002: Standard 与 R5 的 aggregate 并行节省量相同
- Related hypotheses:
  - H-001
  - H-002
- Direction: refutes
- Type: diagnostic-log
- Source: final wire tool-role message deltas
- Prediction or plan link:
  - H-002 parallel savings comparison
- Matched signal:
  - 两侧都通过单 response 多工具减少15次请求
- Correlation keys:
  - pair-001..003
- Raw content:
  ```text
  Standard total tool calls including file changes: 34
  Standard tool-bearing responses: 19
  Standard parallel savings: 34 - 19 = 15

  R5 ordinary tool calls: 51
  R5 taskspace_control calls: 12
  R5 total function calls: 63
  R5 tool-bearing responses: 48
  R5 parallel savings: 63 - 48 = 15
  ```
- Interpretation: R5 后半段虽然常见单工具 response，但 aggregate batching 没有净损失；请求差来自调用总量。
- Time: 2026-07-10 22:56

## Evidence E-003: 12个 control-only response 是稳定结构成本
- Related hypotheses:
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: three R5 rollout traces and `taskspace-control-usage.json`
- Prediction or plan link:
  - H-003 per-run control sequence
- Matched signal:
  - 每轮 `initialize_map=1, finish_node=3`
- Correlation keys:
  - map-1
  - node-1..3
- Raw content:
  ```text
  pair-001 controls: initialize_map, finish_node, finish_node, finish_node
  pair-002 controls: initialize_map, finish_node, finish_node, finish_node
  pair-003 controls: initialize_map, finish_node, finish_node, finish_node
  bind_node=0
  state_commit=0
  control calls total=12
  all controls occupy control-only model responses
  ```
- Interpretation: 三节点简单 Map 的 mandatory tool protocol 固定贡献12次调用和对应采样往返，不是 runtime 隐式重试。
- Time: 2026-07-10 22:56

## Evidence E-004: 17个额外普通动作可按原始命令逐项闭合
- Related hypotheses:
  - H-001
  - H-004
- Direction: supports
- Type: reproduction
- Source: Standard `whale-exec.jsonl` and R5 `rollout.jsonl`
- Prediction or plan link:
  - H-001 ordinary action delta; H-004 alternative explanation
- Matched signal:
  - ordinary function calls Standard=34, R5=51
- Correlation keys:
  - pair-001..003
- Raw content:
  ```text
                    pair-001  pair-002  pair-003  total
  extra inspect         +1        +1        +3       +5
  extra validation/env  +1         0        +4       +5
  patch/read loop        0        +7         0       +7
  ordinary delta        +2        +8        +7      +17

  fixed controls        +4        +4        +4      +12
  total call delta      +6       +12       +11      +29
  parallel-saving delta -1        +1         0        0
  request delta         +5       +13       +11      +29
  ```
- Interpretation: 普通调用放大不是单一机制；第二轮 patch loop 和第三轮测试环境探测贡献最大。
- Time: 2026-07-10 22:56

## Evidence E-005: pre-init 重复来自明确 hard reject，不是读取结果丢失
- Related hypotheses:
  - H-004
- Direction: refutes
- Type: diagnostic-log
- Source: pair-002/pair-003 R5 function_call_output
- Prediction or plan link:
  - H-004 pre-init feedback visibility
- Matched signal:
  - Agent 先调用普通工具，被空 Map 硬规则拒绝，随后 initialize_map 再重发命令
- Correlation keys:
  - `call_00_tChXVxXFkDE8GBMzJUbV3491`
  - `call_00_9rcNmO5zKUqNDRb3x7J62849`
- Raw content:
  ```text
  TaskSpace active task path has no nodes. hard_state: active_task_path_without_nodes.
  TaskSpaceGateRecoveryV1: {"schema_version":"TaskSpaceGateRecoveryV1","allowed":false,"gate_class":"state_machine","reason":"active_task_path_without_nodes","blocking_items":[],"missing_evidence":[]}
  ```
- Interpretation: 两次重复 find/ls 是 Agent 先违反明确硬底线后纠正；第一次调用没有读取内容，因此不是 read 结果被丢失。
- Time: 2026-07-10 22:56

## Evidence E-006: patch 和 pytest 反馈完整进入 history
- Related hypotheses:
  - H-004
- Direction: refutes
- Type: diagnostic-log
- Source: pair-001..003 R5 call/output pairs and cache summaries
- Prediction or plan link:
  - H-004 failure feedback and append-only wire
- Matched signal:
  - 失败/成功输出原文存在；strict-prefix 48/48
- Correlation keys:
  - pair-002 patch call ids
  - pair-001/pair-003 pytest calls
- Raw content:
  ```text
  apply_patch verification failed: Failed to find context '@@ def format_depth() -> str:'
  Success. Updated the following files: M src/call_stack_counter.py
  /home/zhangxu/miniconda3/bin/python: No module named pytest
  R5 strict-prefix: 48/48
  R5 request-2+ cache hit: 97.66%
  ```
- Interpretation: Agent 收到了可行动的失败事实并据此重读/修正；额外动作不能归因于反馈缺失或历史替换。
- Time: 2026-07-10 22:56
