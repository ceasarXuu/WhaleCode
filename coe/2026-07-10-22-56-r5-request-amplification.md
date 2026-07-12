# Problem P-001: R5 TaskSpace 在 G1 正确性样本中请求次数显著高于 Standard
- Status: in-progress-post-j6-6-cadence-observation
- Created: 2026-07-10 22:56
- Updated: 2026-07-12
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
  - E-007
  - E-008
  - E-009
  - E-010
  - E-011
  - E-012
  - E-013
  - E-014
  - E-015
  - E-016
  - E-017
  - E-018
  - E-019
  - E-020
  - E-021
  - E-022
  - E-023
  - E-024
  - E-025
  - E-026
  - E-027
  - E-028
  - E-029
  - E-030
  - E-031
  - E-032
  - E-033
  - E-034
  - E-035
  - E-036
  - E-037
- Ruled out:
  - 相邻请求缓存前缀破坏不是本轮请求放大的原因：R5 strict-prefix 48/48，request-2+ cache hit 97.66%。
  - 工具反馈丢失不是本轮主要原因：pre-init hard reject、pytest failure、apply_patch failure 和成功输出都按原文进入后续 history。
  - aggregate 并行工具承载能力下降不是29次放大的原因：Standard/R5 都因并行工具各少发15次请求。
  - 根任务 `active` 或 Result `unreviewed` 没有在本样本制造额外无工具循环：所有中间 response 都对应至少一个工具调用，且没有重复 bind/state_commit。
- Causal boundary:
  - 12个 Map control 是当前三节点 mandatory-map contract 的确定性结构成本，三轮均可复现。
  - 17个额外普通动作是本轮 treatment-correlated observation，不等于都已证明由 TaskSpace 稳定触发。
  - 其中2个 pre-init find/ls 被空 Map 硬规则拒绝，属于明确的 TaskSpace 特有额外动作；第二轮7个 patch/read 循环和其余环境/检查动作仍需更多 controlled repeats 区分上下文影响与模型采样波动。
  - 三轮首请求都完整包含初始化协议；pre-init 失败不是系统提示或 projection 丢失，而是模型在明确硬状态下仍选择了普通工具。
  - 当前 native tool scheduler 没有为状态工具与后续普通工具提供显式的有序依赖屏障，不能把依赖调用安全地当作普通 parallel tool calls 合并。
  - J2 已补齐有序状态屏障，但 J4 真实运行仍没有产生混合 control/ordinary response；能力存在不等于 Agent 会在依赖边界主动预声明后续调用。
  - `d2cc4b7` 在删除过度设计时同时删掉了必要的机械 API 语义：初始化会建立立即可用的绑定、finish 可原子绑定下一节点、`node_id` 默认当前绑定。J4 的重复 bind 与该变更严格同向。
- Fix criteria:
  - 在用户授权修复后，任何方案必须分别度量固定 control 往返、普通动作数量和并行承载；不得只压低一个总 request 数。
  - 修复验证必须保持 G1 strict-prefix、反馈忠实性、Agent-owned Map 和 correctness。
  - 需要 controlled repeats 证明 request 降低不是模型随机波动。
- Current conclusion: G1 的29次请求差已闭合；J1-J5 已补齐空 Map tool choice、有序 barrier、Agent-authored terminal candidate、结构化初始化映射、重复同名 control承载和显式 ready target原子 finish。hard reject standalone finish被三轮 live run反证并已回撤。详细 trace进一步排除“Agent不知道下一动作”：reasoning明确计划后续 patch/test/final，但工具输出仍停在 finish。Provider A/B证明 thinking+auto可返回多调用；当前模型把 `finish -> ordinary` 识别为需等待结果的跨工具同步边界，而 multi-finish只有显式强调同一响应时才在最小probe出现。最终 live仍为 `multi-control=0`、`chained-finish=0`、`mixed=0`；后续应先做完整 payload controlled A/B，不通过 runtime自动动作、语义注入或 hard cadence gate压指标。
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
  - H-013
  - H-014
  - H-015
  - H-016
  - H-017
  - H-018
  - H-019
- Resolution basis:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
  - E-006
  - E-007
  - E-008
  - E-009
  - E-010
  - E-011
  - E-012
  - E-013
  - E-014
  - E-015
  - E-016
  - E-017
  - E-018
  - E-019
  - E-020
  - E-021
  - E-022
  - E-023
  - E-024
  - E-025
  - E-026
- Close reason:
  - original diagnosis complete; follow-up feedback ambiguity remains open

## Hypothesis H-016: 初始化 key 到 runtime node id 的二次翻译制造可避免的 control retry
- Status: fixed
- Parent: P-001
- Claim: J6 schema 让 Agent 用 `node_key` 初始化节点，却要求后续 finish 改用 runtime 生成的 `next_node_id`；这种同一对象的双重标识迫使 Agent 从工具输出读取映射并翻译，导致 focused sample 首次把 `fix` 当作 node id、control 失败并增加一次 provider request。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - Agent 已经为节点提供稳定 key，状态机只需校验该标识唯一、依赖有效和状态可迁移；再生成一套只供后续 API 使用的 id 没有业务收益。
- Falsifiable predictions:
  - If true: 初始化输出完整进入后续上下文，Agent 明确知道映射但第一次仍使用自己的 key；失败后改用 runtime id 即成功。
  - If false: 初始化映射在 provider history 中缺失/残缺，或失败值并非初始化 key。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对初始化 call/output、失败 finish、下一轮 reasoning 和成功重试按 call id 对账。
  - Signal: `node_id_by_key`、失败参数、失败原文、Agent 下一轮复述、重试参数。
  - Capture method: Docker `rollout.jsonl` 和 performance observation。
  - Event name or marker:
    - `TaskSpaceInitializeMapResultV1`
    - `TaskSpaceControlBatchResultV1`
  - Correlation keys:
    - `call_00_J47dXHwaXiuZP67vEM6u3177`
    - `call_00_Z52kmmfVoa0FhIJ2QnTW8600`
    - `call_00_eZ6wkEG7RVAjI5Oyp1zI5260`
  - Differentiates from:
    - feedback 丢失或 projection 扭曲。
    - runtime 阻止正确 Agent 动作。
    - provider 不支持组合 tool call。
  - Supports if:
    - 映射忠实可见，失败值等于 Agent key，修正为 runtime id 后同一事务成功。
  - Refutes if:
    - 映射不可见，或修正 id 后仍因其他原因失败。
  - Instrumentation status: existing-permanent-observability
  - Instrumentation lifecycle:
    - 保留 control batch 原始输出和 observer control failure 计数。
- Evidence gate: satisfied
- Related evidence:
  - E-027
  - E-028
  - E-029
  - E-030
- Conclusion: fixed by stable Agent-authored node ids; E-031 validates the original failure is absent
- Repair design readiness: ready and authorized by the active J6 implementation request
- Next step: 初始化 schema 直接接收 Agent-authored node id，依赖、current binding 和后续 finish 全程使用同一标识；不保留 key/id 双轨兼容。
- Blocker:
  - none
- Close reason:
  - stable identifier validation passed

## Hypothesis H-017: nested action 降级为任意参数对象导致工具能力语义丢失
- Status: fixed
- Parent: P-001
- Claim: J6 carrier 只枚举 nested tool 名称，却没有透传原工具参数 schema；Agent 因此看不到 `send_message.target` 等必填字段，在复杂 sample 中生成参数不完整的动作并增加失败与请求。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - 组合 carrier 不能以减少 schema 体积为由丢弃原工具调用 contract，否则 Agent 获得的能力语义弱于直接调用同一工具。
- Falsifiable predictions:
  - If true: provider-visible nested `send_message.arguments` 是 unrestricted object；trace 中 nested call 缺少直接工具 schema 要求的 `target`，router 原样返回 missing-field error。
  - If false: nested schema 已包含 `target` required，或失败不是参数缺失。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对最终 tool schema 构造源码、复杂 sample nested call 和原始 output 对账。
  - Signal: nested arguments schema、Agent arguments、handler parse error。
  - Capture method: source inspection + Docker rollout。
  - Event name or marker:
    - `TaskSpaceControlBatchResultV1`
  - Correlation keys:
    - `call_00_Td3tWbz0wwSJmR4kiMCV2539:nested:0`
    - `call_00_gQMI5isRNRVsM2LPb3mI9642:nested:0`
  - Differentiates from:
    - Agent 忽略完整 schema。
    - runtime 重写参数。
    - provider 拒绝 nested schema。
  - Supports if:
    - schema 确实丢失 required 字段，失败原文与缺失字段一致。
  - Refutes if:
    - 原 schema 已透传或 runtime 改写了正确参数。
  - Instrumentation status: existing-permanent-observability
  - Instrumentation lifecycle:
    - 保留 raw nested response 和 nested action schema 单测。
- Evidence gate: satisfied
- Related evidence:
  - E-031
  - E-032
- Conclusion: fixed by embedding each visible function tool's original parameter schema; E-033 validates the missing-target failure is absent
- Repair design readiness: ready and authorized by the active J6 implementation request
- Next step: `taskspace_control.actions[].arguments` 直接引用可见 function tool 的原始 `parameters` schema；custom tool 继续保留原始 input 形态；不做摘要或重新解释。
- Blocker:
  - none
- Close reason:
  - nested tool schema fidelity validation passed

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

## Hypothesis H-005: pre-init 普通工具调用来自初始化协议没有进入模型上下文
- Status: refuted
- Parent: P-001
- Claim: pair-002 和 pair-003 首次调用 find/ls，是因为模型首请求没有收到必须先执行 `initialize_map` 的协议。
- Layer: sub-cause
- Factor relation: single
- Depends on:
  - H-004
- Rationale:
  - 如果初始化说明缺失，模型按 Standard 习惯先检查仓库是合理行为。
- Falsifiable predictions:
  - If true: 失败轮首请求 history 中缺少 `active_task_path_without_nodes` 或 `initialization_contract`。
  - If false: 三轮收到相同初始化硬状态，其中一轮正确初始化、两轮忽略后被硬规则拒绝。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查三轮首请求 developer items 和首个 function call。
  - Signal: 初始化 marker、首工具名及参数。
  - Capture method: R5 `rollout.jsonl` 原始 response items。
  - Correlation keys:
    - pair id
    - first provider response
  - Supports if:
    - 失败轮缺少初始化协议。
  - Refutes if:
    - 协议存在且未被改写。
- Evidence gate: satisfied
- Related evidence:
  - E-007
- Conclusion: refuted
- Repair design readiness: ready
- Next step: 不增加语义提示；评估让 provider tool choice/visibility 与现有 hard state 一致。
- Blocker:
  - repair not authorized
- Close reason:
  - all three first requests contained the explicit initialization contract

## Hypothesis H-006: 每个 finish 独占请求是未合并 bind 操作造成的
- Status: refuted
- Parent: P-001
- Claim: `finish_node` 之后还需要单独 bind 下一节点，导致每个节点边界多一次请求。
- Layer: sub-cause
- Factor relation: single
- Depends on:
  - H-003
- Rationale:
  - lifecycle 工具如果只完成节点、不推进 binding，会形成可避免的双重控制往返。
- Falsifiable predictions:
  - If true: trace 中每个 finish 后存在 `bind_node`，或 finish output 没有下一节点 binding。
  - If false: finish 已通过 `next_node_id` 原子完成当前节点并绑定下一节点，但工具结果仍需下一次模型采样。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对三轮 finish 参数、输出和 bind 计数。
  - Signal: `next_node_id`、`Next node ... bound`、`bind_node=0`。
  - Capture method: R5 `rollout.jsonl` call/output pair。
  - Correlation keys:
    - finish call id
    - node id
  - Supports if:
    - 存在额外 bind 往返。
  - Refutes if:
    - finish 已原子推进 binding。
- Evidence gate: satisfied
- Related evidence:
  - E-008
  - E-009
- Conclusion: refuted
- Repair design readiness: not applicable
- Next step: 将成本定位为 native tool-result sampling barrier，而不是 bind 实现缺口。
- Blocker:
  - none
- Close reason:
  - all transitions already finish and bind atomically

## Hypothesis H-007: 当前 native tool scheduler 缺少状态变更后的有序多步骤执行能力
- Status: confirmed
- Parent: P-001
- Claim: Agent 不能在一次 provider response 中可靠表达 `initialize/finish -> dependent ordinary tool`，因为当前调用被当作同批 in-flight tools，普通工具 TaskSpace preflight 在串行执行锁之前运行，没有“前一步成功后按最新状态校验下一步”的协议。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-003
- Rationale:
  - 普通 parallel tools 适合互不依赖的调用；状态迁移和下一节点动作是顺序依赖，必须有明确 barrier、逐步 preflight 和首错停止语义。
- Falsifiable predictions:
  - If true: `taskspace_control` 标记为 non-parallel，但普通工具的 TaskSpace preflight 位于 execution lock 之前；native transport 没有逐项 latest-state sequence contract。
  - If false: 当前 scheduler 已保证同一 response 内按输出顺序执行、每步基于前一步结果重新校验并在失败后跳过后续调用。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查 tool registry、parallel dispatcher、native transport selector 和已有 sequence executor。
  - Signal: `supports_parallel_tool_calls=false`、preflight/lock 顺序、`NativeTools` 固定返回、旧 action sequence 的 ordered loop。
  - Capture method: 源码审计。
  - Correlation keys:
    - tool call order
    - call id
    - node binding/lease
  - Supports if:
    - native scheduler 缺少 barrier，而旧 sequence path 证明机械顺序执行在架构上可行。
  - Refutes if:
    - native path 已提供同等能力。
- Evidence gate: satisfied
- Related evidence:
  - E-008
  - E-010
- Conclusion: confirmed
- Repair design readiness: ready, but implementation requires user authorization
- Next step: 设计 native-tools barrier，不恢复禁用 native tools 的旧 action-contract transport。
- Blocker:
  - repair not authorized
- Close reason:
  - not closed

## Hypothesis H-008: J4 的重复 bind 来自工具简化时删除了机械动作语义
- Status: refuted
- Parent: P-001
- Claim: J4 中 Agent 在 `initialize_map` 后重复绑定当前节点、在每次 `finish_node` 后单独绑定下一节点，主要因为 `d2cc4b7` 删除了工具 schema 中初始化即时绑定、finish 原子切换和默认当前节点的机械用法说明。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 状态机作为工具必须准确暴露调用契约；删除任务策略是正确的，但不能同时删除参数已经实现的机械效果。
- Falsifiable predictions:
  - If true: 变更前同类运行会使用 `finish_node(next_node_id)` 且没有 `bind_node`；变更后会系统性出现单独 bind。
  - If false: 两个版本的 schema 相同，或行为回归早于该提交，或初始化输出没有向 Agent 返回当前节点。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对比 G1/J4 rollout、`d2cc4b7` 前后 tool schema 和初始化原始输出。
  - Signal: `next_node_id`、`bind_node` 数量、参数 description 和完整 tool description。
  - Capture method: git diff 加 Docker rollout call/output 对账。
  - Correlation keys:
    - commit
    - run id
    - node id
  - Differentiates from:
    - Provider 不支持多工具调用。
    - 初始化结果被 projection 丢失或改写。
    - runtime 主动要求重复绑定。
  - Supports if:
    - G1 三轮 `bind_node=0`，J4 多个样本稳定为 `bind_node=N`，且中间提交精确删除对应机械说明。
  - Refutes if:
    - 行为与契约变更不相关，或 Agent 已收到同等明确的机械语义。
  - Instrumentation status: sufficient
- Evidence gate: satisfied
- Related evidence:
  - E-009
  - E-011
  - E-012
  - E-013
  - E-015
  - E-016
- Conclusion: 工具契约缺失是确认存在的产品缺陷，但“它是重复 bind 的主要或充分原因”已被 fix validation 反证。新 schema 已送达且 Agent 理解默认当前 binding，仍产生4次成功独立 bind；不能继续把所有 control 放大归因于该缺失。
- Repair design readiness: repair completed; causal benefit claim rejected
- Next step: 保留已恢复的真实机械契约；转向初始化反馈映射歧义及 `next_node_id` 未采用的独立诊断。
- Blocker:
  - none
- Close reason:
  - fix validation contradicted the primary-cause prediction

## Hypothesis H-009: mixed barrier 为零是依赖边界选择，不是 Provider 多工具能力缺失
- Status: confirmed
- Parent: P-001
- Claim: J4 真实运行没有生成 `taskspace_control -> ordinary tool` 混合 response，是因为状态迁移后的普通动作依赖迁移结果，而一次 function-calling response 在生成全部 calls 时尚未收到中间工具结果；不是 Provider 禁用了多工具调用。
- Layer: mechanism
- Factor relation: single
- Depends on:
  - H-007
- Rationale:
  - J2 只让 runtime 能安全执行已声明序列，不能改变模型生成一轮 calls 时看不到中间结果的事实。
- Falsifiable predictions:
  - If true: 同一运行中独立 ordinary calls 仍会批量出现；模型/provider 配置允许 parallel calls；只有状态边界 mixed 为0。
  - If false: Standard 和 R5 的独立普通调用也全部退化为单调用，或 provider payload 禁止 parallel calls。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对比 Standard/R5 tool-bearing responses，并核对 provider model 配置和 J2 runtime path。
  - Signal: ordinary batching savings、`supports_parallel_tool_calls`、mixed barrier count。
  - Capture method: performance observer、provider wire 和源码审计。
  - Correlation keys:
    - response index
    - tool call id
    - run id
  - Differentiates from:
    - 工具能力整体关闭。
    - scheduler 无法执行有序 barrier。
    - tool result feedback 丢失。
  - Supports if:
    - 独立调用继续成批返回，而所有真实状态迁移边界都选择重新采样。
  - Refutes if:
    - provider 根本不能返回多个 calls，或模型在相同契约下稳定生成 mixed 序列。
  - Instrumentation status: permanent-observability-candidate
- Evidence gate: satisfied for current runs; not a universal model limitation claim
- Related evidence:
  - E-011
  - E-012
  - E-014
- Conclusion: 当前数据确认 provider 能力存在、独立批处理正常；mixed=0 是 Agent 在依赖边界的实际选择。不能据此假设 DeepSeek 原生 tool loop 不稳定，也不能要求 runtime 自动补动作。
- Repair design readiness: not applicable as runtime repair
- Next step: 将 mixed barrier 保留为能力观测项，不再把 `control-only <= 1` 作为强制收益门禁；优先消除由契约缺失产生的冗余 bind。
- Blocker:
  - none
- Close reason:
  - scoped mechanism diagnosed

## Hypothesis H-010: 初始化映射反馈方向歧义直接导致 node key 被当作 node id
- Status: confirmed
- Parent: P-001
- Claim: `initialize_map` 返回的紧凑文本 `node_ids=[node_key=node_id]` 没有显式字段边界，Agent 将映射方向读反，随后把 node key 传给只接受生成 id 的 `finish_node.node_id` 和 `bind_node.node_id`，各制造一次失败和重采样。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-008
- Rationale:
  - 工具反馈必须忠实且无歧义；对人可推断的 `key=value` 文本不等于对模型稳定明确的结构化机械结果。
- Falsifiable predictions:
  - If true: 原始 reasoning 会明确把 `node-1` 与 node key 的映射方向说反，失败调用参数会使用 node key，runtime 则返回 id 不存在或不是当前节点。
  - If false: Agent 使用的是生成的 `node-N` id，或错误发生在映射反馈进入上下文之前。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对账初始化 output、紧随其后的 reasoning、失败 control 参数与原始 hard error。
  - Signal: `node_ids=[key=node-N]`、Agent 映射复述、`node_id=key`、`lifecycle_target_not_current`/`transition_rejected`。
  - Capture method: fix-validation Docker rollout 原始 call/output/reasoning 链。
  - Correlation keys:
    - initialize call id
    - failed control call id
    - node key
    - generated node id
  - Differentiates from:
    - 新 tool schema 未进入 provider payload。
    - runtime 自动改变当前 binding。
    - projection 丢失初始化结果。
  - Supports if:
    - Agent 明确读反映射并用错误 key 调用，两个错误都由 runtime 原样指出。
  - Refutes if:
    - 参数是正确 id，或错误前没有收到映射。
  - Instrumentation status: sufficient
- Evidence gate: satisfied
- Related evidence:
  - E-015
  - E-016
  - E-017
  - E-018
- Conclusion: confirmed and fix-validated；本轮2次失败 control 和对应2次额外 provider requests 可直接归因于反馈格式歧义。结构化反馈上线后错误归零，Agent 正确使用全部生成 id，并恢复3次原子 next binding。
- Repair design readiness: repair complete
- Next step: 保留 `TaskSpaceInitializeMapResultV1` 作为稳定机械反馈 schema；继续观察复杂样本，不把单样本收益外推为全局结论。
- Blocker:
  - none
- Close reason:
  - original symptom absent in fix-validation run

## Hypothesis H-011: 显式目标 finish 缺少无 binding 原子执行能力
- Status: confirmed
- Parent: P-001
- Claim: Agent 已明确提供 ready node 的 `node_id` 时，`finish_node` 仍要求该节点事先成为 current binding，导致连续登记多个已完成节点时必须额外 bind，并在 binding 已释放时产生 `no_current_node_binding` 失败。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-007
- Rationale:
  - `finish_node(node_id=...)` 已表达 Agent 对目标节点的明确选择；当没有 current binding、目标 ready、依赖完成且无租约冲突时，claim target 与 finish 是同一控制动作的机械执行，不需要 runtime 做任务语义判断。
- Falsifiable predictions:
  - If true: live trace 中 standalone finish 释放 binding 后，显式 finish 下一个 ready node会先报 `no_current_node_binding`；手工 bind 后同一 finish成功。
  - If false: 显式 finish 已能在无 binding 时执行，或失败来自依赖未完成、租约冲突、目标不存在等其他硬状态。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对 advisory run 的 finish/bind/output 顺序与 `record_main_node_lifecycle_result` 入口校验对账。
  - Signal: `finish node-2 -> binding none -> finish node-3 -> no_current_node_binding -> bind node-3 -> finish node-3 success`。
  - Capture method: Docker rollout 原始 call/output加 runtime 源码审计。
  - Event name or marker:
    - `TaskSpaceGateRecoveryV1.reason=no_current_node_binding`
    - `finish_node`
    - `bind_node`
  - Correlation keys:
    - call id
    - node id
    - response index
  - Differentiates from:
    - Provider 不支持重复 control。
    - finish 反馈丢失或被 projection 改写。
    - 目标节点依赖未完成。
  - Supports if:
    - 目标 node-3 已因 node-2 完成变为 ready，唯一首个拒绝原因仍是无 current binding，bind 后成功。
  - Refutes if:
    - 失败先发生在依赖、租约或 target 校验，或显式 finish 实际已自动 claim。
  - Instrumentation status: permanent-observability-candidate
- Evidence gate: satisfied
- Related evidence:
  - E-022
  - E-023
- Conclusion: confirmed and fix-validated。`record_main_node_lifecycle_result` 在读取显式目标状态前先强制 `current_main_node_id`，并要求 target=current；advisory live trace 精确复现该顺序。修复仅在 current binding为空时，对 Agent显式指定且可绑定的目标执行同事务 claim+finish；成功、pending失败无副作用和同响应相邻 finish测试全部通过。
- Repair design readiness: repair complete
- Next step: 保留 multi-control/chained-finish观测；不将单轮总请求下降外推为稳定收益。
- Blocker:
  - none
- Close reason:
  - original mechanical rejection absent in fix-validation tests

## Hypothesis H-012: hard reject standalone nonterminal finish 能降低请求成本
- Status: refuted
- Parent: P-001
- Claim: 在执行前拒绝没有同响应 follow-up 的 nonterminal finish，会促使 Agent 改用多 finish 或 finish+next action，从而减少 control-only response。
- Layer: repair-validation
- Factor relation: single
- Depends on:
  - H-007
- Rationale:
  - 候选方案曾将非终态 finish 后继续动作设为 cadence hard rule。
- Falsifiable predictions:
  - If true: 引导到达后 rejection只产生至多一次学习成本，随后 chained/mixed response增加且总 requests下降。
  - If false: Agent重复触发拒绝、制造无意义 follow-up过门禁，或请求数和失败数上升。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对三轮逐步增强 hard gate 的 Docker run 比较 rejects、control failures、mixed calls和 follow-up内容。
  - Signal: cadence reject次数、无意义 ordinary call、requests、correctness。
  - Capture method: performance observation与 rollout call/output重建。
  - Correlation keys:
    - run id
    - call id
    - response index
  - Differentiates from:
    - Provider 不支持重复 finish。
    - runtime sequence executor 无法承载多 barrier。
  - Supports if:
    - repeated-control probe失败，或 live run很快稳定使用有意义 chained calls。
  - Refutes if:
    - provider probe通过但 Agent重复拒绝并提交 no-op follow-up。
  - Instrumentation status: removed-hard-gate-retained-advisory
- Evidence gate: satisfied
- Related evidence:
  - E-019
  - E-020
  - E-021
- Conclusion: refuted。Provider 明确支持同名 control 重复调用；最强引导 run仍出现6次 cadence reject，随后 Agent用3个无意义 `echo` 过门禁。hard gate已删除，合法 finish正常提交，性能工具改为非阻断统计 standalone nonterminal finish。
- Repair design readiness: repair reverted
- Next step: 保留能力说明和节奏观测，不恢复 cadence拒绝。
- Blocker:
  - none
- Close reason:
  - live validation contradicted benefit prediction

## Hypothesis H-013: standalone finish 是因为 Agent 不知道下一动作
- Status: refuted
- Parent: P-001
- Claim: Agent 在调用 finish 时尚未形成后续 patch/test/final动作，因此只能等待下一轮重新规划。
- Layer: mechanism
- Factor relation: single
- Depends on:
  - H-012
- Rationale:
  - 如果下一动作未知，单独 finish 是合理的自然工具循环，而不是调用承载偏好。
- Falsifiable predictions:
  - If true: finish前 reasoning只描述状态提交，不会同时明确后续普通动作。
  - If false: reasoning明确写出“更新节点并运行测试/继续修复”，但响应仍只有 finish call。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对齐每个 reasoning item、同响应 tool calls和下一请求首个动作。
  - Signal: finish前文本中的后续动作是否等于下一请求动作。
  - Capture method: final Docker rollout逐响应重建。
  - Correlation keys:
    - response index
    - finish call id
  - Differentiates from:
    - 模型已知动作但把状态工具当同步边界。
  - Supports if:
    - 后续动作未出现于 finish前 reasoning。
  - Refutes if:
    - 后续动作已明确出现且下一请求按该动作执行。
  - Instrumentation status: sufficient
- Evidence gate: satisfied
- Related evidence:
  - E-024
- Conclusion: refuted。两处 reasoning分别明确写出“更新节点并运行测试”和“完成任务”，但 tool response只包含 finish；下一请求随即执行此前已经写出的动作。
- Repair design readiness: not applicable
- Next step: 不通过增加任务规划提示解决。
- Blocker:
  - none
- Close reason:
  - live reasoning contradicted missing-plan prediction

## Hypothesis H-014: 模型把 finish 到 ordinary tool 视为必须等待结果的同步边界
- Status: confirmed
- Parent: P-001
- Claim: DeepSeek会批量提交独立工具，但对 `finish_node -> apply_patch/exec_command` 这类跨工具状态依赖链，即使 reasoning和提示明确要求同一响应，仍倾向只返回第一个状态调用。
- Layer: mechanism
- Factor relation: primary
- Depends on:
  - H-007
- Rationale:
  - ChatCompletions用 `parallel_tool_calls=true` 表达多调用；runtime的有序 barrier是 Whale内部扩展。模型可以理解文字说明，但其原生调用策略仍可能只把可并行或同类预声明动作放入一个响应。
- Falsifiable predictions:
  - If true: independent reads继续批量；明确要求的重复 control可以批量；现实语义下 finish+ordinary即使 reasoning说“同一响应做两件事”仍只返回 finish。
  - If false: thinking/auto模式不支持多调用，或显式 finish+ordinary稳定返回两个 calls。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对 thinking-enabled + auto执行独立、重复 control、自然 mixed和显式 mixed A/B。
  - Signal: tool call count、tool names、reasoning中声明的计划。
  - Capture method: live rollout加不记录正文/密钥的最小 Provider probes。
  - Correlation keys:
    - probe name
    - tool call order
  - Differentiates from:
    - client截断第二个 call。
    - thinking模式禁用多调用。
    - cadence说明未进入上下文。
  - Supports if:
    - auto+thinking可返回两个重复 controls，但 realistic mixed只返回 control一个。
  - Refutes if:
    - 所有模式都只能返回一个，或 realistic mixed返回两个。
  - Instrumentation status: follow-up-probe-needed-in-benchmark
- Evidence gate: satisfied
- Related evidence:
  - E-019
  - E-024
  - E-025
- Conclusion: confirmed for current model/configuration。独立6 reads同响应、auto+thinking重复 control为2 calls；但 natural/explicit `finish -> exec_command` 三个probe均只返回一个 control，reasoning仍明确声称会在同响应执行两步。
- Repair design readiness: diagnostic conclusion only
- Next step: 不让 runtime自动补 ordinary action；若继续优化，先把 realistic mixed A/B纳入正式 provider probe并验证工具描述的机械示例是否有效。
- Blocker:
  - none
- Close reason:
  - scoped mechanism diagnosed

## Hypothesis H-015: multi-finish 未采用与机械用法显著性不足有关
- Status: investigating
- Parent: P-001
- Claim: 工具描述中的 `Prefer chaining` 是长期稳定但低显著性的能力说明；模型自然执行时沿用一次工具一轮的习惯，只有任务输入明确要求“in one response”时才生成两个 finish calls。
- Layer: mechanism
- Factor relation: contributing
- Depends on:
  - H-014
- Rationale:
  - 同类 finish之间没有 ordinary feedback依赖，Provider和runtime都能承载；自然任务与显式单响应指令的输出不同。
- Falsifiable predictions:
  - If true: 自然“finish A then finish B”返回1 call，显式“in one response”返回2 calls；机械示例可能提高采用率。
  - If false: 两种提示输出一致，或真实上下文即使明确单响应仍稳定1 call。
- Diagnostic evidence plan:
  - Prediction or clause under test: 用完整真实 tool schema、相同历史和状态快照做 natural/explicit paired probe。
  - Signal: chained finish count。
  - Capture method: 后续正式 provider A/B fixture，至少controlled repeats。
  - Correlation keys:
    - prompt variant
    - repeat
  - Differentiates from:
    - 跨工具同步边界。
    - Provider能力缺失。
  - Supports if:
    - 只有显式单响应variant稳定产生两个 finish。
  - Refutes if:
    - 完整上下文下两侧都不产生或都产生。
  - Instrumentation status: planned
- Evidence gate: partial
- Related evidence:
  - E-025
- Conclusion: 当前最小probe支持，但尚未用完整真实history和controlled repeats确认。不能据此立即增强全局提示。
- Repair design readiness: not ready
- Next step: 先做完整 payload A/B，不修改 runtime gate。
- Blocker:
  - realistic controlled repeats absent
- Close reason:
  - not closed

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

## Evidence E-007: 三轮首请求均完整包含初始化协议
- Related hypotheses:
  - H-005
- Direction: refutes
- Type: diagnostic-log
- Source: three R5 `rollout.jsonl`
- Prediction or plan link:
  - H-005 bootstrap context visibility
- Matched signal:
  - 三轮均出现 activation hard state、blank-map hard state 和 initialize contract
- Correlation keys:
  - pair-001..003
  - first provider response
- Raw content:
  ```text
  TaskSpace mode is now active.
  hard_state: ordinary tools and multi-agent actions require an active TaskSpace task path, current node binding, and lease.
  hard_state: active_task_path_without_nodes
  initialization_contract: taskspace_control(action=initialize_map)

  pair-001 first call: taskspace_control(initialize_map)
  pair-002 first call: exec_command(find ...)
  pair-003 first call: exec_command(ls -la)
  ```
- Interpretation: 模型知道硬状态所需事实；两次错误是明确指令下的动作选择失败，不是 projection 丢失、裁剪或扭曲。
- Time: 2026-07-10 23:24

## Evidence E-008: native control tool 是独占执行工具但没有依赖序列契约
- Related hypotheses:
  - H-006
  - H-007
- Direction: supports
- Type: source-inspection
- Source: `tools/src/tool_registry_plan.rs`, `core/src/tools/parallel.rs`, `core/src/session/turn.rs`
- Prediction or plan link:
  - H-007 scheduler ordering
- Matched signal:
  - `taskspace_control` 注册为 `supports_parallel_tool_calls=false`
  - 当前 transport 固定为 `NativeTools`
  - function calls 进入 `FuturesOrdered`，但普通工具的 `prepare_taskspace_tool_call` 位于 parallel execution lock 之前
- Correlation keys:
  - function call id
  - response item order
- Raw content:
  ```text
  tool_registry_plan.rs:453-456  taskspace_control supports_parallel_tool_calls=false
  turn.rs:1164/1171             transport=NativeTools
  parallel.rs:121-123           TaskSpace preflight
  parallel.rs:151-168           execution lock and dispatch
  ```
- Interpretation: non-parallel 标记只能约束执行互斥，不能构成“前一步状态提交成功后再校验下一步”的有序事务；把 finish 和下一普通工具直接并列仍存在旧状态校验/归属风险。
- Time: 2026-07-10 23:24

## Evidence E-009: finish 已原子完成并绑定下一节点
- Related hypotheses:
  - H-006
- Direction: refutes
- Type: diagnostic-log
- Source: three R5 `rollout.jsonl`
- Prediction or plan link:
  - H-006 finish/bind decomposition
- Matched signal:
  - 前两个 finish 都携带 `next_node_id`，三轮 `bind_node=0`
- Correlation keys:
  - node-1..3
  - finish call id
- Raw content:
  ```text
  finish_node(node-1, next_node_id=node-2) -> node-1 completed; node-2 bound
  finish_node(node-2, next_node_id=node-3) -> node-2 completed; node-3 bound
  finish_node(node-3) -> node-3 completed
  bind_node calls: 0
  ```
- Interpretation: 当前四个 control 的成本不能再通过合并 create/bind 消除；剩余边界是每个 control output 后的 provider resampling。
- Time: 2026-07-10 23:24

## Evidence E-010: 旧 action sequence 证明机械有序执行可行但当前 native path 未使用
- Related hypotheses:
  - H-007
- Direction: supports
- Type: source-inspection
- Source: `core/src/session/turn.rs`
- Prediction or plan link:
  - H-007 ordered execution feasibility
- Matched signal:
  - 旧 `taskspace-action-sequence-v1` 定义按列表顺序、每步读取最新状态、首错停止；当前 selector 固定返回 `NativeTools`
- Correlation keys:
  - sequence index
  - tool call id
- Raw content:
  ```text
  turn.rs:1178-1187  sequence order/latest state/stop-on-failure contract
  turn.rs:13831-13930 sequential action execution
  turn.rs:1164/1171  current request transport remains NativeTools
  ```
- Interpretation: 有序多步骤不是不可实现；应把 barrier 能力下沉到通用 native tool scheduler，保留每个工具的权限、沙箱、原始反馈和 trace，而不是恢复会禁用 native tools、引入独立 JSON 协议的旧 transport。
- Time: 2026-07-10 23:24

## Evidence E-011: G1 真实运行曾稳定使用 finish 的原子下一节点绑定
- Related hypotheses:
  - H-008
  - H-009
- Direction: supports
- Type: diagnostic-log
- Source: `target/r5-g1-repeats/count-call-stack/20260710-210444-351`
- Prediction or plan link:
  - H-008 pre-change behavior
- Matched signal:
  - 三轮均为 `initialize_map=1, finish_node=3, bind_node=0`
- Correlation keys:
  - pair-001..003
  - node-1..3
- Raw content:
  ```text
  initialize_map(current_node_key=explore)
    -> current_node=node-1
  finish_node(node-1, next_node_id=node-2)
    -> node-1 completed; node-2 bound
  finish_node(node-2, next_node_id=node-3)
    -> node-2 completed; node-3 bound
  finish_node(node-3)
  bind_node=0
  ```
- Interpretation: DeepSeek 已在相同产品和同类样本中正确使用过原子状态动作，不能把 J4 行为归因于模型天然不会组合。
- Time: 2026-07-11

## Evidence E-012: J4 同类运行稳定退化为每节点单独 bind 和 finish
- Related hypotheses:
  - H-008
  - H-009
- Direction: supports
- Type: diagnostic-log
- Source: `target/r5-j4-batching-contract/count-call-stack/20260711-060333-715` and J4 performance observations
- Prediction or plan link:
  - H-008 post-change behavior
- Matched signal:
  - 固定三节点样本为 `initialize_map=1, bind_node=3, finish_node=3`，且所有 control 独占 response
- Correlation keys:
  - pair-001
  - node-1..3
- Raw content:
  ```text
  initialize_map -> current_node=node-1
  bind_node(node-1)
  finish_node(node-1)
  bind_node(node-2)
  finish_node(node-2)
  bind_node(node-3)
  finish_node(node-3)

  J4 fixed sample: controls=7, mixed barrier=0
  multi-file sample: controls=13, mixed barrier=0
  subscription sample: controls=18, mixed barrier=0
  ```
- Interpretation: 初始化原始输出完整包含 `current_node`，但 Agent 没有把它理解为“已建立立即可用的 binding”；行为是系统性契约理解回归，不是单轮随机失误。
- Time: 2026-07-11

## Evidence E-013: `d2cc4b7` 精确删除了原子动作的机械使用说明
- Related hypotheses:
  - H-008
- Direction: supports
- Type: source-inspection
- Source: `git diff d2cc4b7^ d2cc4b7 -- tools/src/taskspace_tool.rs`
- Prediction or plan link:
  - H-008 schema regression
- Matched signal:
  - 参数仍存在且 runtime 行为仍实现，但 schema 不再说明即时绑定、默认当前节点和 finish 原子切换
- Correlation keys:
  - commit `d2cc4b7`
  - `current_node_key`
  - `node_id`
  - `next_node_id`
- Raw content:
  ```text
  before current_node_key:
    Required for initialize_map. node_key to bind for immediate work.
  after current_node_key:
    Required current node key for initialize_map.

  before finish_node:
    node_id is optional and defaults mechanically to the current main node binding.
    It may also bind next_node_id ... atomically.
  after tool description:
    no per-action lifecycle semantics
  ```
- Interpretation: R5 拆除过度设计时误删了工具自描述所必需的 API 契约。该层可以描述参数的真实机械效果，但不应注入任务策略。
- Time: 2026-07-11

## Evidence E-014: Provider 多工具能力开启且独立 ordinary calls 继续批处理
- Related hypotheses:
  - H-009
- Direction: supports
- Type: source-and-runtime-inspection
- Source: `models-manager/models.json`, J0 provider probe, J4 Standard/R5 traces
- Prediction or plan link:
  - H-009 provider capability alternative
- Matched signal:
  - `deepseek-v4-flash.supports_parallel_tool_calls=true`；J0 返回有序多 calls；J4 独立 reads/tests 仍同 response 批量出现
- Correlation keys:
  - model slug
  - response index
  - tool call id
- Raw content:
  ```text
  supports_parallel_tool_calls: true
  J0 required probe: first_step -> second_step in one response
  G1/J4 ordinary-only responses: multiple independent calls observed
  J4 mixed taskspace_control/ordinary responses: 0
  ```
- Interpretation: 多工具能力没有被关闭。状态迁移后的动作需要依赖前一步执行结果，而模型生成同一 response 的全部 calls 时看不到该结果；J2 让 runtime 可以安全承载这种预声明序列，但不保证 Agent 会选择它。
- Time: 2026-07-11

## Evidence E-015: 机械 action contract 修复已进入生产 schema 和 provider 请求
- Related hypotheses:
  - H-008
  - H-010
- Direction: supports
- Type: fix-validation
- Source: commit `f0db9d7`, codex-tools tests, fix-validation provider wire
- Prediction or plan link:
  - H-008 repair delivery
- Matched signal:
  - 字段和总说明均恢复真实机械效果；focused tests/build 通过；新旧 run tools hash 不同且新 hash 全程稳定
- Correlation keys:
  - commit `f0db9d7`
  - tools hash `bf8e99d8ec4e5e03cc4f9425583b629e9bb3a6b134498937caa58de0de4ac492`
  - request index 1..21
- Raw content:
  ```text
  current_node_key: referenced initial node is bound as current in the same state transition
  node_id: finish_node defaults to the current binding
  next_node_id: finish and next binding commit atomically

  codex-tools focused tests: 2 passed
  cargo build -p codex-cli --bin whale --locked: passed
  old tools hash: ec805e8674c21906b280de3bb6d8043cdf60e382307054cc083341286df09ca8
  new tools hash: bf8e99d8ec4e5e03cc4f9425583b629e9bb3a6b134498937caa58de0de4ac492
  new strict tools hash coverage: 21/21
  ```
- Interpretation: 不能用“修复未构建、未送达或被后续请求替换”解释本轮行为；操作语义暴露缺失本身已经工程修复。
- Time: 2026-07-11

## Evidence E-016: 单轮 fix validation 未恢复原子 next binding，并暴露映射读反
- Related hypotheses:
  - H-008
  - H-009
  - H-010
- Direction: mixed
- Type: fix-validation
- Source: `target/r5-j4-mechanical-contract/count-call-stack/20260711-181112-154`
- Prediction or plan link:
  - H-008 behavioral recovery; H-010 mapping ambiguity
- Matched signal:
  - correctness、terminal candidate 和 cache 通过；`next_node_id` 使用为0；Agent 明确读反 node key/id 映射并触发两次 control failure
- Correlation keys:
  - pair-001/right
  - initialize call `call_00_ZjIwVsqH6MkZejMKIqb00533`
  - failed finish `call_00_CK1srH0E3M1Ar4ljv1vR7160`
  - failed bind `call_00_OpADfnHyBZOXkvZUIAIc6779`
- Raw content:
  ```text
  Standard: solved, requests=7, tools=13, wall=15.04s
  TaskSpace: solved, requests=21, tools=13, controls=12, wall=43.47s
  TaskSpace map: 5 nodes, 4 edges, 5 results, open=0
  controls: initialize_map=1, finish_node=6, bind_node=5
  mixed barrier=0, terminal_candidate=1, extra_final_request=0
  request-2+ cache hit: Standard=94.64%, TaskSpace=96.33%

  initialize output:
    current_node=node-1 node_ids=[read-readme-and-tests=node-1,...]
  Agent reasoning:
    "The output maps node-1=read-readme-and-tests."
  failed calls:
    finish_node(node_id="read-readme-and-tests") -> lifecycle_target_not_current
    bind_node(node_id="inspect-codebase") -> transition_rejected / node does not exist
  ```
- Interpretation: 新契约让 Agent 正确保留初始化 binding，并知道 finish 可默认当前节点，但没有恢复 `finish_node(next_node_id)`。两次额外失败明确来自反馈映射方向被读反；其余独立 bind 的原因仍不能仅凭本轮归因。
- Time: 2026-07-11

## Evidence E-017: 初始化反馈改为方向显式的结构化机械结果
- Related hypotheses:
  - H-010
- Direction: supports
- Type: fix-implementation
- Source: commit `6c0153c`, taskspace_control handler and focused tests
- Prediction or plan link:
  - H-010 repair design
- Matched signal:
  - 删除手工 `key=id` 拼接，输出 schema 分离 current key/id 和完整 key-to-id object
- Correlation keys:
  - commit `6c0153c`
  - schema `TaskSpaceInitializeMapResultV1`
- Raw content:
  ```json
  {
    "schema_version": "TaskSpaceInitializeMapResultV1",
    "action": "initialize_map",
    "status": "initialized",
    "task_id": "task-1",
    "map_id": "map-1",
    "current_node_key": "read_readme_and_tests",
    "current_node_id": "node-1",
    "node_id_by_key": {
      "read_readme_and_tests": "node-1",
      "inspect_implementation": "node-2"
    }
  }
  ```
- Interpretation: 结果只表达 runtime 已提交的机械事实，不提示下一动作、不选择节点、不改写 Agent 语义。JSON 序列化避免分隔符和映射方向歧义。
- Time: 2026-07-11

## Evidence E-018: 结构化映射复验恢复 bind=0 和 N+1 controls
- Related hypotheses:
  - H-008
  - H-009
  - H-010
- Direction: supports
- Type: fix-validation
- Source: `target/r5-j4-explicit-init-mapping/count-call-stack/20260711-183707-628`
- Prediction or plan link:
  - H-010 original symptom removal and atomic-next observation
- Matched signal:
  - 4节点 Map 无 key/id 错误、无 bind_node，前三次 finish 均携带正确 next_node_id
- Correlation keys:
  - pair-001/right
  - initialize call `call_00_Yd4Vg0KeBADKaOjzWtfj5764`
  - node-1..4
- Raw content:
  ```text
  Standard: solved, requests=5, ordinary tools=10, wall=12.35s
  TaskSpace: solved, requests=12, ordinary tools=13, controls=5, wall=24.61s
  TaskSpace map: 4 nodes, 3 edges, 4 results, open=0
  controls: initialize_map=1, finish_node=4, bind_node=0
  transitions:
    finish node-1 next_node_id=node-2
    finish node-2 next_node_id=node-3
    finish node-3 next_node_id=node-4
    finish node-4 final_candidate=<Agent-authored>
  control hard errors=0
  mixed barrier=0, terminal_candidate=1, extra_final_request=0
  request-2+ cache hit: Standard=92.71%, TaskSpace=93.43%
  input tokens: Standard=36,393, TaskSpace=96,949
  ```
- Interpretation: H-010 原始错误已消失，且原子 next binding 恢复。R5 仍有5个 control-only responses 和3个额外 ordinary calls，因此总 request/cost 尚未与 Standard 收敛；该剩余不能再归因于映射反馈。
- Time: 2026-07-11

## Evidence E-019: Provider 支持同一响应重复调用 taskspace_control
- Related hypotheses:
  - H-012
- Direction: refutes
- Type: provider-probe
- Source: `target/r5-j5-provider-probe/provider-capability.json`
- Prediction or plan link:
  - H-012 provider capability alternative
- Matched signal:
  - HTTP 200，同一响应按顺序返回两次 `taskspace_control`
- Correlation keys:
  - probe `ordered_repeated_control_calls`
- Raw content:
  ```text
  tool_names: taskspace_control, taskspace_control
  actions: finish_first, finish_second
  tool_call_count: 2
  ```
- Interpretation: 多 finish 没有发生不能归因于 Provider 不允许重复同名工具；runtime sequence也可按 barrier顺序承载。
- Time: 2026-07-11

## Evidence E-020: 三轮 hard gate 未形成稳定 chained finish
- Related hypotheses:
  - H-012
- Direction: refutes
- Type: fix-validation
- Source: `target/r5-j5-chained-finish`、`target/r5-j5-bound-finish`、`target/r5-j5-guided-finish`
- Prediction or plan link:
  - H-012 benefit prediction
- Matched signal:
  - R5 requests分别17/15/17；cadence rejects分别2/2/6；control failures分别3/2/7
- Correlation keys:
  - 三个 run id
- Raw content:
  ```text
  194736-657: requests=17 rejects=2 control_failures=3
  200043-908: requests=15 rejects=2 control_failures=2
  200756-733: requests=17 rejects=6 control_failures=7
  ```
- Interpretation: 更强的硬拒绝和提示没有稳定降低请求，反而增加失败反馈和纠错采样。
- Time: 2026-07-11

## Evidence E-021: Agent 用无意义 ordinary call 绕过 cadence hard gate
- Related hypotheses:
  - H-012
- Direction: refutes
- Type: diagnostic-log
- Source: `target/r5-j5-guided-finish/count-call-stack/20260711-200756-733`
- Prediction or plan link:
  - H-012 meaningful follow-up prediction
- Matched signal:
  - 6次 cadence reject 后出现3个仅用于满足 follow-up 的 `echo "follow-up after finishing node-X"`
- Correlation keys:
  - pair-001/right
  - finish call id
- Raw content:
  ```text
  finish_node -> cadence reject
  finish_node + exec_command(echo "follow-up after finishing node-X") -> allowed
  ```
- Interpretation: runtime门禁无法创造有意义的下一动作，只会诱导 Agent制造形式合规的 no-op；该规则越过状态机底线并污染上下文。
- Time: 2026-07-11

## Evidence E-022: Advisory run 暴露显式目标 finish 的机械能力缺口
- Related hypotheses:
  - H-011
  - H-012
- Direction: supports
- Type: diagnostic-log-and-source
- Source: `target/r5-j5-advisory-finish/count-call-stack/20260711-201839-033` and `core/src/action_map/runtime.rs`
- Prediction or plan link:
  - H-011 no-binding explicit target chain
- Matched signal:
  - 两侧 solved；hard gate/no-op消失；R5仍有7个 standalone finish、3次 control failure、3次 bind
- Correlation keys:
  - pair-001/right
  - node-2/node-3
- Raw content:
  ```text
  finish node-2 -> success; current binding released
  finish node-3 -> no_current_node_binding
  bind node-4 -> target_node_dependencies_incomplete
  finish node-3(next=node-4) -> no_current_node_binding
  bind node-3 -> success
  finish node-3(next=node-4) -> success

  Standard: solved, requests=11, tools=17, wall=20.17s
  R5: solved, requests=19, tools=12, controls=12, wall=35.07s
  R5 map: 6 nodes, 5 edges, 6 results, open=0
  ```
- Interpretation: 删除 hard gate恢复了 Agent所有权，但工具仍不能把 Agent显式选择的 ready target在无 binding时原子 claim+finish。源码先检查 current binding，再检查显式 target，和 trace 的失败顺序一致。
- Time: 2026-07-11

## Evidence E-023: 原子显式 finish 修复通过事务测试和 Docker 回归
- Related hypotheses:
  - H-011
  - H-012
- Direction: supports
- Type: fix-validation
- Source: commit `d0f35ca`, focused tests, `target/r5-j5-atomic-finish/count-call-stack/20260711-203035-327`
- Prediction or plan link:
  - H-011 explicit ready target success and hard-state failure atomicity
- Matched signal:
  - ready target无 binding直接完成；pending target拒绝后无 binding/lease残留；同响应相邻 finish完成两个依赖节点
- Correlation keys:
  - `explicit_ready_target_is_claimed_and_finished_without_separate_bind`
  - `rejected_explicit_finish_does_not_leave_an_implicit_binding`
  - `adjacent_finish_calls_claim_successive_ready_targets`
- Raw content:
  ```text
  focused runtime tests: passed
  ActionMap scenario tests: 7 passed
  adjacent finish fixture: provider requests=2, nodes completed=2

  final Docker Standard: solved, requests=8, tools=13, wall=19.11s
  final Docker R5: solved, requests=10, tools=12, controls=5, wall=27.11s
  R5: control failures=0, bind_node=0, terminal extra request=0
  R5 cadence: control-only=5, multi-control=0, chained-finish=0, mixed=0
  R5 map: nodes=4, edges=0, results=4, open=0
  ```
- Interpretation: 工具能力缺口已修复且不放松 pending/lease等硬状态。真实样本正确性和错误归零，但 Agent仍逐响应提交 finish；能力完成不等于行为采用，且4节点0边与上一轮拓扑不同，不能把 requests 19 -> 10全部归因于修复。
- Time: 2026-07-11

## Evidence E-024: finish 前 reasoning 已明确包含下一动作
- Related hypotheses:
  - H-013
  - H-014
- Direction: refutes
- Type: diagnostic-log
- Source: `target/r5-j5-atomic-finish/count-call-stack/20260711-203035-327/pair-001/right/artifacts/rollout.jsonl`
- Prediction or plan link:
  - H-013 missing-plan prediction
- Matched signal:
  - Agent在 finish前明确计划后续动作，但同响应只返回 control
- Correlation keys:
  - request 4/6/9
  - node-1..4
- Raw content:
  ```text
  request 4 reasoning: bug understood; let me fix it
  response: finish node-1 only
  request 5: apply_patch

  request 6 reasoning: fix applied; update the node and run tests and validation
  response: finish node-2 only
  request 7: pytest + CLI

  request 9 reasoning: validation passes; let me finish the task
  response: finish node-3 only
  request 10: finish node-4(final_candidate)
  ```
- Interpretation: 单独 finish不是因为缺少后续计划；模型在工具生成阶段将已知计划切到下一次tool-result round。
- Time: 2026-07-11

## Evidence E-025: thinking-enabled auto模式支持多调用但 realistic mixed仍单调用
- Related hypotheses:
  - H-014
  - H-015
- Direction: supports
- Type: provider-probe
- Source: 2026-07-11 sanitized live Provider A/B probe
- Prediction or plan link:
  - H-014 thinking/tool-choice alternatives
- Matched signal:
  - thinking+auto在显式能力probe中可返回2个重复 controls和 control+patch；接近真实状态的 control+exec probes均只返回1个 control
- Correlation keys:
  - probe variant
- Raw content:
  ```text
  thinking_auto_repeated_control: 2 calls
  thinking_auto_minimal_control_patch: 2 calls

  natural_update_then_test: reasoning says finish then tests; 1 control call
  explicit_same_response_finish_then_exec: reasoning says both in one response; 1 control call
  hard_state_with_preflight_clarification: reasoning says ordered pair; 1 control call

  natural_finish_two_nodes: 1 control call
  explicit_in_one_response_two_controls: 2 control calls
  ```
- Interpretation: thinking模式、auto tool choice和Provider transport均不是缺失点。模型区分了可一次预声明的同类/独立调用与需要前一步状态结果的跨工具调用；multi-finish还受“同一响应”显著性影响。
- Time: 2026-07-11

## Evidence E-026: 当前 Map 结构本身制造了四个生命周期边界
- Related hypotheses:
  - H-003
  - H-015
- Direction: supports
- Type: runtime-state
- Source: final Docker performance observation and initialize_map call
- Prediction or plan link:
  - fixed control overhead
- Matched signal:
  - Agent创建4节点0边Map，其中独立 `final_synthesis` 节点没有 ordinary work；每节点各提交一次 finish
- Correlation keys:
  - map-1
  - node-1..4
- Raw content:
  ```text
  nodes=4, edges=0, results=4
  controls=initialize_map + 4 finish_node
  control-only responses=5
  node-4 title=Final answer
  ```
- Interpretation: 即使任务工作只需要 read/edit/validate，Agent-authored Map仍增加了独立 final节点和对应状态边界。runtime不应自动合并，但该结构解释了为什么终态candidate只省掉最终文本请求，没有省掉 node-3 -> node-4迁移请求。
- Time: 2026-07-11

## Evidence E-027: J6 focused sample 仅一次 control failure 恰好对应一次请求差
- Related hypotheses:
  - H-016
- Direction: supports
- Type: reproduction
- Source: `target/r5-j6-schema-first/count-call-stack/count-call-stack/20260712-035847-201/performance-observation.json`
- Prediction or plan link:
  - H-016 request amplification prediction
- Matched signal:
  - Standard/R5 均 solved；requests 7/8；R5 control failures=1，terminal extra request=0。
- Correlation keys:
  - pair-001/right
- Raw content:
  ```text
  standard: requests=7, solved=true
  taskspace: requests=8, solved=true, control_failure=1
  carriers: init+actions=1, finish+actions=3, finish+end=1
  terminal_extra_request=0
  ```
- Interpretation: J6 已消除终态额外请求和独立 finish，但唯一 control retry 仍使请求数比 Standard 多1。
- Time: 2026-07-12

## Evidence E-028: 初始化映射完整且原样进入下一轮上下文
- Related hypotheses:
  - H-016
- Direction: supports
- Type: diagnostic-log
- Source: `target/r5-j6-schema-first/count-call-stack/count-call-stack/20260712-035847-201/pair-001/right/artifacts/rollout.jsonl`
- Prediction or plan link:
  - H-016 feedback-presence prediction
- Matched signal:
  - 初始化 control output 同时包含 `fix=node-2`，并保留两个 nested read 的完整输出。
- Correlation keys:
  - `call_00_J47dXHwaXiuZP67vEM6u3177`
- Raw content:
  ```text
  node_id_by_key={"fix":"node-2","inspect":"node-1","validate":"node-3"}
  nested:0 ls success
  nested:1 README content success
  ```
- Interpretation: 本次失败不能归因于初始化结果或嵌套工具反馈丢失、裁剪或重写。
- Time: 2026-07-12

## Evidence E-029: Agent 使用初始化 key 后收到忠实的硬状态错误
- Related hypotheses:
  - H-016
- Direction: supports
- Type: diagnostic-log
- Source: same J6 focused rollout
- Prediction or plan link:
  - H-016 dual-identifier prediction
- Matched signal:
  - Agent 提交 `next_node_id="fix"`；runtime 返回 `TaskSpace next node 'fix' does not exist`，没有执行 nested patch。
- Correlation keys:
  - `call_00_Z52kmmfVoa0FhIJ2QnTW8600`
- Raw content:
  ```text
  finish_then_actions.finishes[0].next_node_id="fix"
  status=state_failed
  success=false
  reason=TaskSpace next node `fix` does not exist.
  ```
- Interpretation: runtime 在硬状态边界正确拒绝不存在的 id，反馈语义忠实；问题位于工具标识 contract，而不是错误传播。
- Time: 2026-07-12

## Evidence E-030: Agent 明确复述映射后改用 runtime id 即成功
- Related hypotheses:
  - H-016
- Direction: supports
- Type: diagnostic-log
- Source: same J6 focused rollout
- Prediction or plan link:
  - H-016 correction prediction
- Matched signal:
  - 失败后 reasoning 明确写出 `inspect -> node-1, fix -> node-2, validate -> node-3`；下一 call 使用 `node-2`，finish 和 nested patch 同批成功。
- Correlation keys:
  - `call_00_eZ6wkEG7RVAjI5Oyp1zI5260`
- Raw content:
  ```text
  reasoning: fix -> node-2
  next_node_id="node-2"
  status=completed
  finish success=true
  apply_patch success=true
  ```
- Interpretation: 反馈有效且 Agent 能纠正；消除双重标识可机械地删除这次无价值纠错，而无需 runtime 解释、引导或推断 Agent 意图。
- Time: 2026-07-12

## Evidence E-031: 稳定 node id 复跑消除原始映射失败
- Related hypotheses:
  - H-016
- Direction: supports
- Type: fix-validation
- Source: `target/j6-contract-c/count-call-stack/count-call-stack/20260712-041525-466`
- Prediction or plan link:
  - H-016 stable identifier fix criterion
- Matched signal:
  - R5 solved，control failure=0；Agent 初始化 `n1-inspect..n4-final` 后全程复用相同 id；终态一次完成三个节点。
- Correlation keys:
  - pair-001/right
- Raw content:
  ```text
  requests=8
  initialize_then_actions=1
  finish_then_actions=1
  finish_then_end=1
  control_failures=0
  terminal_extra_request=0
  ```
- Interpretation: key/runtime-id 双轨删除后，原始 `next_node_id=key` 失败未复现；runtime 未增加语义判断。
- Time: 2026-07-12

## Evidence E-032: 复杂 sample 的 nested send_message 缺失原工具必填参数
- Related hypotheses:
  - H-017
- Direction: supports
- Type: diagnostic-log-and-source
- Source: `target/j6-complex-a/order-pipeline/multi-file-order-pipeline/20260712-041646-435` and `tools/src/taskspace_tool.rs`
- Prediction or plan link:
  - H-017 schema-transmission prediction
- Matched signal:
  - nested function schema 使用 generic unrestricted object；Agent 两次只提供 `message`，handler 两次原样返回 missing `target`。
- Correlation keys:
  - `call_00_Td3tWbz0wwSJmR4kiMCV2539:nested:0`
  - `call_00_gQMI5isRNRVsM2LPb3mI9642:nested:0`
- Raw content:
  ```text
  actions[0]={"tool_name":"send_message","arguments":{"message":"..."}}
  output=failed to parse function arguments: missing field `target`
  ```
- Interpretation: 这是 carrier 构造层对工具能力语义的确定性丢失，不应归因于 Agent 智能，也不应通过 runtime 后置纠正。
- Time: 2026-07-12

## Evidence E-033: 原工具参数 schema 透传后复杂 sample 不再产生参数缺失
- Related hypotheses:
  - H-017
- Direction: supports
- Type: fix-validation
- Source: commit `81d2702` and `target/j6-complex-b/order-pipeline/multi-file-order-pipeline/20260712-042255-022`
- Prediction or plan link:
  - H-017 nested schema fidelity fix criterion
- Matched signal:
  - nested schema equality单测通过；复杂 R5 solved，protocol/state failure=0，未出现 `send_message.target` 或其他 required-field 缺失。
- Correlation keys:
  - pair-001/right
- Raw content:
  ```text
  taskspace_tool nested parameter equality: passed
  requests=12
  protocol_failures=0
  state_failures=0
  nested_action_failures=1 (apply_patch context mismatch)
  terminal_extra_request=0
  ```
- Interpretation: 工具能力 contract 已忠实进入 carrier；剩余 nested failure 是普通 patch 上下文不匹配，原始错误完整进入 batch output，不属于 schema 语义丢失。
- Time: 2026-07-12

## Hypothesis H-018: 精确 nested schema 在 control 内重复两次并与顶层工具叠加造成固定请求体放大
- Status: fixed
- Parent: P-001
- Claim: `taskspace_control` 将完整 ordinary-action union 分别内联到 init/finish 两个分支，同时 ordinary tools 继续顶层暴露，导致同一参数 schema 在活跃请求中物理出现三次。
- Layer: root-cause
- Factor relation: additive
- Depends on:
  - H-017
- Rationale:
  - 参数语义完整性修复是正确的，但重复序列化不是语义要求；同一 function schema 应在 control 内定义一次并引用。
- Falsifiable predictions:
  - If true: R5 每轮非 message wire bytes 稳定高于 Standard，且 `81d2702` 后相对 generic nested schema 再固定增加；源码存在两次 `actions.clone()`。
  - If false: wire 增量来自动态 messages，或 nested union 只序列化一次。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对三代 run 的最终 wire、tools hash和 schema构造源码做独立对账。
  - Signal: per-request non-message bytes、schema出现次数、commit前后固定增量。
  - Capture method: provider wire trace + source inspection。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
  - Correlation keys:
    - tools hash `0858cd2c...`
    - tools hash `1d1ee594...`
    - tools hash `2427b0f3...`
  - Differentiates from:
    - projection accumulation。
    - 个别大工具输出。
    - cache miss导致总 input 增加。
  - Supports if:
    - Standard约21.69 KB、generic R5约30.82 KB、exact R5约48.44 KB，且源码两次克隆同一 union。
  - Refutes if:
    - non-message bytes不稳定或 schema不重复。
  - Instrumentation status: existing-permanent-observability
  - Instrumentation lifecycle:
    - 保留 provider final-wire bytes/tools hash；增加 schema serialized-byte gate。
- Evidence gate: satisfied
- Related evidence:
  - E-034
  - E-035
- Conclusion: confirmed; exact schema fidelity and physical schema deduplication are separate requirements
- Repair design readiness: ready and authorized by user
- Next step: closed；active 两份必要能力成本留给后续整体能力设计观察，不通过收编普通工具处理。
- Blocker:
  - none
- Close reason:
  - `$defs/$ref` 与 blank-map filter 已落地；DeepSeek 双样本接受 schema，active non-message bytes 从约48.44 KB降至约36.35 KB。

## Hypothesis H-019: prefix telemetry 忽略 tool choice 导致冷启动边界被误报为完整前缀保持
- Status: fixed
- Parent: P-001
- Claim: provider wire comparator 只比较 tools hash 和 message LCP；Req1 `named taskspace_control` 到 Req2 `auto` 仍被记为 `prefix_preserved=true`，无法解释冷 schema 的前两轮0命中。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - H-018
- Rationale:
  - cache hit是provider事实；本地只能忠实记录完整请求形状、历史相似度和实际hit/miss，不能把message append-only等同于cache shape不变。
- Falsifiable predictions:
  - If true: comparator结构不保存tool choice；cold run呈0/0/high，warm run呈high/partial/high；Req1到Req2仅70ms且tool choice变化。
  - If false: comparator已包含tool choice，或Req2与Req1完全同形。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对 comparator字段、cold/warm同hash run和request时间线对账。
  - Signal: `tool_choice_kind/name`、cached tokens、request间隔、tools hash历史出现次数。
  - Capture method: source inspection + provider wire/cache trace。
  - Event name or marker:
    - `provider.chat_wire_prefix_preserved`
    - `provider.chat_wire_request_terminal`
  - Correlation keys:
    - `j6-contract-b`
    - `j6-contract-c`
    - `j6-complex-b`
  - Differentiates from:
    - 每轮持续缓存破坏。
    - tools hash在同一run内变化。
    - message history被替换。
  - Supports if:
    - cold run前两轮0、第三轮高命中；warm run首轮高、第二轮部分；当前 comparator忽略tool choice。
  - Refutes if:
    - 同shape重复请求持续0命中且工具选择无变化。
  - Instrumentation status: existing-observability-incomplete
  - Instrumentation lifecycle:
    - 升级wire/cache trace schema并保留为永久观测。
- Evidence gate: satisfied
- Related evidence:
  - E-036
  - E-037
- Conclusion: confirmed; the cache itself recovers, but telemetry overstates prefix preservation
- Repair design readiness: ready and authorized by user
- Next step: closed；继续把V3字段作为永久观测，不增加runtime行为。
- Blocker:
  - none
- Close reason:
  - wire trace v2与cache summary v3已落地；cold run正确分类2个warmup candidate，紧接warm run为0个zero hit。

## Evidence E-034: exact nested schema 使每轮固定非 message payload 增长17.62 KB
- Related hypotheses:
  - H-018
- Direction: supports
- Type: diagnostic-log
- Source: J6 Standard/generic R5/exact R5 provider cache traces
- Prediction or plan link:
  - H-018 fixed wire overhead prediction
- Matched signal:
  - Standard约21.69 KB/request；generic R5约30.82 KB/request；exact R5约48.44 KB/request。
- Correlation keys:
  - `j6-complex-a/pair-001/left`
  - `j6-complex-a/pair-001/right`
  - `j6-complex-b/pair-001/right`
- Raw content:
  ```text
  standard avg non-message bytes = 21691.2
  generic R5 avg non-message bytes = 30824.7
  exact R5 avg non-message bytes = 48443.8
  ```
- Interpretation: 增量每轮稳定存在，不是个别历史或工具输出尖峰；`81d2702` fidelity修复新增约17.62 KB/request。
- Time: 2026-07-12

## Evidence E-035: schema构造源码将同一action union内联两次
- Related hypotheses:
  - H-018
- Direction: supports
- Type: source
- Source: `tools/src/taskspace_tool.rs`
- Prediction or plan link:
  - H-018 physical duplication prediction
- Matched signal:
  - `function_action_schema` clone原参数；`initialize_then_actions_schema`和`finish_then_actions_schema`分别执行`actions.clone()`；registry同时保留顶层ordinary tools。
- Correlation keys:
  - lines 44-54, 249-305, 387-403
- Raw content:
  ```text
  arguments = tool.parameters.clone()
  initialize actions.items = actions.clone()
  finish actions.items = actions.clone()
  ```
- Interpretation: 三重暴露由确定性序列化结构产生；去重不需要损失任何工具参数语义。
- Time: 2026-07-12

## Evidence E-036: cold/warm同hash运行稳定复现前两轮缓存差异
- Related hypotheses:
  - H-019
- Direction: supports
- Type: reproduction
- Source: `j6-contract-b` and `j6-contract-c`
- Prediction or plan link:
  - H-019 cold-shape prediction
- Matched signal:
  - cold run hit率为0%、0%、91.28%；相邻warm run为98.48%、41.76%、96.64%。
- Correlation keys:
  - tools hash `1d1ee594...`
- Raw content:
  ```text
  cold: req1=0 req2=0 req3=.912846
  warm: req1=.984832 req2=.417640 req3=.966378
  ```
- Interpretation: cache在第三轮恢复；Req2低命中与首次auto shape和缓存持久化时序相关，不是持续破坏。
- Time: 2026-07-12

## Evidence E-037: comparator忽略tool choice且Req2仅晚于Req1完成70ms
- Related hypotheses:
  - H-019
- Direction: supports
- Type: source-and-runtime
- Source: `core/src/provider_wire_trace.rs` and `j6-complex-b` request events
- Prediction or plan link:
  - H-019 telemetry-gap prediction
- Matched signal:
  - `WireRequestShape`仅保存tools hash/messages；Req1 named，Req2 auto；Req1 completed到Req2 started间隔70ms，但trace仍记录prefix preserved。
- Correlation keys:
  - logical-1
  - logical-2
- Raw content:
  ```text
  req1 completed=1783801382095
  req2 started=1783801382165
  delta=70ms
  ```
- Interpretation: 观测口径把message append-only误当完整cache shape保持；必须纳入tool choice并单列warmup事实。
- Time: 2026-07-12

## Evidence E-038: local ref 与 blank-map filter 消除第三份 schema 暴露
- Related hypotheses:
  - H-018
- Direction: supports
- Type: fix-validation
- Source: commit `a7e47de` and two J6.5 provider wire traces
- Prediction or plan link:
  - H-018 physical deduplication exit gate
- Matched signal:
  - control 参数只在 `$defs.ordinaryAction` 定义一次 exact union；init/finish 均使用本地 `$ref`；blank request tools=1，active tools=13。
- Correlation keys:
  - blank tools hash `e648401a...`
  - active tools hash `1320c089...`
- Raw content:
  ```text
  J6 exact active non-message ~= 48443.8 bytes
  J6.5 blank-map non-message = 19568 bytes
  J6.5 active non-message ~= 36350 bytes
  ```
- Interpretation: 三份物理暴露已收敛；active 仍有顶层 ordinary tools 与 control 内 nested union 两份必要能力，不通过 generic schema 或 runtime 收编进一步压缩。
- Time: 2026-07-12

## Evidence E-039: cold run 将两个新 shape 的零命中正确分类为 warmup candidate
- Related hypotheses:
  - H-019
- Direction: supports
- Type: fix-validation
- Source: `target/r5-j6-5-schema-cache/count-call-stack/count-call-stack/20260712-060218-891`
- Prediction or plan link:
  - H-019 cache classification exit gate
- Matched signal:
  - Req1为单control named shape，Req2为ordinary+control auto shape；两者cached=0且此前均未出现，V3统计zero=2、warmup=2、same-shape-zero=0。
- Correlation keys:
  - cache shape `a8dd4510...`
  - cache shape `9e51e5ad...`
- Raw content:
  ```text
  prefix preserved = 9/10
  message prefix preserved across req1->req2 = true
  tool choice transition = 1
  cache shape transition = 1
  zero / warmup / same-shape-zero = 2 / 2 / 0
  ```
- Interpretation: telemetry不再把message append-only等同完整请求shape稳定；冷启动事实与同shape异常已可机械区分。
- Time: 2026-07-12

## Evidence E-040: 相同 binary/tool shape 的后续复杂运行首轮已恢复部分缓存
- Related hypotheses:
  - H-019
- Direction: supports
- Type: warm-run-validation
- Source: `target/r5-j6-5-schema-cache/order-pipeline/multi-file-order-pipeline/20260712-060409-492`
- Prediction or plan link:
  - H-019 warm-shape recovery prediction
- Matched signal:
  - TaskSpace Req1/Req2 hit率为13.17%/36.75%，后续active请求约91.9%-98.2%；zero、warmup和same-shape-zero均为0。
- Correlation keys:
  - cache shape `a8dd4510...`
  - cache shape `9e51e5ad...`
- Raw content:
  ```text
  req1=.131732
  req2=.367475
  req3=.945298
  req7=.982298
  req8=.982343
  zero / warmup / same-shape-zero = 0 / 0 / 0
  ```
- Interpretation: 缓存本身可以在后续运行复用；此前0命中不是持续性本地缓存破坏。provider命中仍为best-effort，不能由runtime保证。
- Time: 2026-07-12

## Hypothesis H-020: J6.5简单样本的错误路径由TaskSpace读取或失败反馈失真直接造成
- Status: refuted
- Parent: P-001
- Claim: R5在已读取源码后访问不存在目录、随后生成失败patch，是因为nested tool结果或失败语义没有完整进入下一轮上下文。
- Layer: root-cause-candidate
- Factor relation: alternative
- Depends on:
  - H-017
- Rationale:
  - R4/R5历史上同类重复动作曾由反馈丢失造成，因此必须优先排查语义链，不能先归因于Agent智能。
- Falsifiable predictions:
  - If true: Req2源码输出缺失或被摘要替换，Req3无法指出真实文件；patch失败输出缺失、改写或被runtime拒绝。
  - If false: 原始源码和签名完整存在，Agent明确复述真实路径/bug后仍自行构造错误路径或错误patch，错误反馈随后完整进入上下文并触发纠正。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对Req2 control output、Req3 reasoning/call、Req5 patch input/output和下一轮纠正逐项对账。
  - Signal: 原始路径、函数签名、Agent复述、tool arguments、handler output。
  - Capture method: rollout response items + exact payload scanner。
  - Event name or marker:
    - `function_call_output`
    - `provider.chat_wire_prefix_preserved`
  - Correlation keys:
    - `call_00_aOSwgmpktigB7nBoIk0X8020`
    - `call_00_h9oZD9G0IJf8Kv8vEcCV9194`
    - `call_00_lEBdU5x9Hki0RcwudnTz5450`
  - Differentiates from:
    - Agent自主的探索性路径假设。
    - Agent生成patch时遗漏函数返回注解。
  - Supports if:
    - 原始结果或错误反馈不可见、不完整或被改写。
  - Refutes if:
    - 原始结果和失败反馈完整，且Agent在调用前已明确知道正确事实。
  - Instrumentation status: existing-permanent-observability
  - Instrumentation lifecycle:
    - 保留raw control history、exact payload scan和rollout调用/反馈。
- Evidence gate: satisfied
- Related evidence:
  - E-041
  - E-042
- Conclusion: refuted；两次错误均为Agent-authored低级错误，没有发现TaskSpace直接语义缺陷
- Repair design readiness: not-applicable
- Next step: 通过多样本统计观察较大上下文/carrier是否间接提高低级错误率，不增加runtime约束。
- Blocker:
  - none
- Close reason:
  - direct semantic-transmission mechanism contradicted by raw trace

## Hypothesis H-021: 当前TaskSpace单请求Input增量主要来自active control工具schema
- Status: confirmed
- Parent: P-001
- Claim: J6.5去除第三份暴露后，active request相对Standard的主要固定增量仍是额外`taskspace_control` schema；bootstrap、Map/control自然历史是第二增量，请求数只放大总量而不改变单请求结构。
- Layer: cost-attribution
- Factor relation: additive
- Depends on:
  - H-018
- Rationale:
  - cold cache只改变cached/uncached归属，不增加input token本身；必须按wire固定区与message区分账。
- Falsifiable predictions:
  - If true: active R5 non-message bytes固定比Standard高约14.67 KB；同进度message bytes只高约4-5 KB；复杂样本即使R5请求更少，总Input仍因每请求固定增量而较高。
  - If false: 主要增量来自projection重复、历史替换或单个异常message，non-message差异应很小或不稳定。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对两组paired run逐request拆分provider payload的message/non-message bytes并与usage对账。
  - Signal: tools count、non-message bytes、message bytes、input tokens、active projection count。
  - Capture method: provider wire trace + exact payload scan + cache usage。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
    - `exact_payload_scan`
  - Correlation keys:
    - Standard tools hash `0858cd2c...`
    - R5 active tools hash `1320c089...`
  - Differentiates from:
    - stale projection accumulation。
    - cache miss本身制造input token。
    - 额外request导致的总量放大。
  - Supports if:
    - active non-message稳定约36.35 KB vs Standard约21.69 KB，且projection count恒为1。
  - Refutes if:
    - message区占绝大多数固定差异或projection重复出现。
  - Instrumentation status: existing-permanent-observability
  - Instrumentation lifecycle:
    - 保留wire byte split与projection uniqueness scanner。
- Evidence gate: satisfied
- Related evidence:
  - E-043
- Conclusion: confirmed；active单请求wire差异约四分之三来自control schema，约四分之一来自TaskSpace消息历史
- Repair design readiness: observation-only
- Next step: 后续能力设计评估active双重能力暴露的收益/成本，不牺牲工具语义或收编Agent动作。
- Blocker:
  - none
- Close reason:
  - attribution confirmed; optimization remains a separate design decision

## Hypothesis H-022: 当前projection是固定epoch base，剩余成本来自冗长base而非逐请求Map累积
- Status: confirmed
- Parent: P-001
- Claim: J6.6单轮中的active projection只在epoch起点生成一次，后续Map变化通过append-only control journal表达；因此projection没有逐请求累积，但冗长blank base和activation被每次请求重复计入Input。
- Layer: context-layout-and-cost
- Factor relation: additive
- Depends on:
  - H-021
- Rationale:
  - 必须区分“当前Map快照未刷新导致语义丢失”和“event-sourced epoch base + delta journal”。前者需要动态更新，后者应压缩base并保留稳定prefix。
- Falsifiable predictions:
  - If true: 9次请求的projection message bytes/hash完全相同；rollout只有一次projection budget event；初始化和finish delta在后续raw control call/output中完整可见。
  - If false: projection hash会随Map变化，或Agent看不到初始化/finish delta，或provider payload包含多份projection。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对每个wire request读取projection message hash，并对runtime projection trace、raw control journal和context生成guard交叉验证。
  - Signal: message index/hash、projection budget event count、control call/output pair、epoch snapshot presence guard。
  - Capture method: provider wire trace + rollout + session/runtime code path。
  - Event name or marker:
    - `ContextProjectionV1 epoch snapshot:`
    - `projection_budget`
    - `taskspace_control`
  - Correlation keys:
    - run `20260712-065907-459`
    - projection content hash `c30d837f...`
  - Differentiates from:
    - stale projection替换失败。
    - projection每请求累积。
    - control delta丢失。
  - Supports if:
    - 固定单份projection + 完整append-only state journal同时成立。
  - Refutes if:
    - projection实际动态变化、重复出现，或state delta不可见。
  - Instrumentation status: implemented-and-validated
  - Instrumentation lifecycle:
    - 保留runtime projection budget trace；修复benchmark extractor使其可直接分账。
- Evidence gate: satisfied
- Related evidence:
  - E-050
- Conclusion: confirmed；当前语义布局是固定epoch base + 原始delta journal，不应通过逐请求刷新完整Map破坏prefix cache
- Repair design readiness: implemented
- Next step: 在复杂样本继续观察请求采用率；不改为逐请求动态projection。
- Blocker:
  - none
- Close reason:
  - extractor、最小epoch base、稀疏populated snapshot和成功ack均已实现并完成focused live validation

## Evidence E-041: Agent收到完整源码后仍自行假设不存在的package目录
- Related hypotheses:
  - H-020
- Direction: refutes
- Type: raw-trace
- Source: J6.5 count-call-stack right rollout
- Prediction or plan link:
  - H-020 source-visibility clause
- Matched signal:
  - Req2 control output完整包含`src/call_stack_counter.py`正文；Req3 reasoning明确复述该文件和bug，随后为了检查`__main__.py`自行调用不存在的`/workspace/src/call_stack_counter/`。
- Correlation keys:
  - `call_00_aOSwgmpktigB7nBoIk0X8020`
  - `call_00_h9oZD9G0IJf8Kv8vEcCV9194`
- Raw content:
  ```text
  source: def format_depth() -> str:
  reasoning: The source file src/call_stack_counter.py has a format_depth() function...
  call: ls -la /workspace/src/call_stack_counter/
  output: No such file or directory
  ```
- Interpretation: Agent知道真实文件仍做了额外package形态探查；这是Agent生成的错误探索动作，不是读取结果丢失。
- Time: 2026-07-12

## Evidence E-042: patch失败来自Agent遗漏返回注解且错误反馈完整透传
- Related hypotheses:
  - H-020
- Direction: refutes
- Type: raw-trace-and-standard-control
- Source: J6.5 count-call-stack paired rollout
- Prediction or plan link:
  - H-020 patch feedback clause
- Matched signal:
  - R5 patch使用`@@ def format_depth():`，真实签名是`def format_depth() -> str:`；Standard同轮使用完整签名并成功。R5 handler原样返回找不到上下文，Agent随后读取精确文件并纠正。
- Correlation keys:
  - R5 `call_00_lEBdU5x9Hki0RcwudnTz5450`
- Raw content:
  ```text
  R5 patch: @@ def format_depth():
  Standard patch: @@ def format_depth() -> str:
  feedback: apply_patch verification failed: Failed to find context 'def format_depth():'
  ```
- Interpretation: schema、参数路由和失败反馈均忠实；失败点是Agent生成的patch上下文不精确。
- Time: 2026-07-12

## Evidence E-043: active wire增量约77%来自control schema固定区
- Related hypotheses:
  - H-021
- Direction: supports
- Type: wire-cost-attribution
- Source: J6.5 count-call-stack and multi-file-order-pipeline paired provider wire traces
- Prediction or plan link:
  - H-021 fixed/non-message split
- Matched signal:
  - Standard active non-message约21.69 KB，R5约36.35 KB，固定差约14.67 KB；相近进度message区通常额外约4-5 KB；所有R5请求active projection count=1。
- Correlation keys:
  - tools hash `0858cd2c...`
  - tools hash `1320c089...`
- Raw content:
  ```text
  order req4: standard messages=9641 non-message=21687
              R5 messages=13908 non-message=36354
  delta: messages=4267, non-message=14667
  count req8: standard messages=14772 non-message=21697
              R5 messages=19201 non-message=36355
  delta: messages=4429, non-message=14658
  ```
- Interpretation: 当前单请求Input差距是结构性的，但主项是model-visible工具能力schema，不是projection累积；额外request会再次携带该固定成本并放大总Input。
- Time: 2026-07-12

## Evidence E-044: 三次并行重复未稳定复现原始路径和patch错误
- Related hypotheses:
  - H-020
- Direction: supports
- Type: parallel-reproduction
- Source: R5-J6.6 three right-only count-call-stack runs
- Prediction or plan link:
  - H-020 Agent-error variability prediction
- Matched signal:
  - 三次均solved；原始package-path error只在run-3出现，patch-context error为0/3；run-1出现一次无`PYTHONPATH` CLI，run-2出现一次malformed initial control。
- Correlation keys:
  - `20260712-064055-854`
  - `20260712-064055-857`
  - `20260712-064055-898`
- Raw content:
  ```text
  requests = 7 / 9 / 9
  package-path error = 0 / 0 / 1
  patch-context error = 0 / 0 / 0
  business success = 3 / 3
  ```
- Interpretation: 原始两类错误不是稳定TaskSpace机制故障；低级动作类型随采样变化。后续优先降低schema/context负担，不增加Runtime语义约束。
- Time: 2026-07-12

## Evidence E-045: active普通工具重复表达已消除
- Related hypotheses:
  - H-021
- Direction: supports
- Type: implementation-and-live-wire
- Source: R5-J6.6 count-call-stack paired run
- Prediction or plan link:
  - `docs/v0.0.5/build-R5/20-r5-j6-6-active-single-tool-expression-plan.md`
- Matched signal:
  - active control serializer不含`ordinaryAction`、`tool_name`或ordinary arguments；8个active request的tools hash恒定，non-message payload为22,488-22,503 bytes。
- Correlation keys:
  - commit `fd9f759`
  - run `20260712-065907-459`
  - active tools hash `8e8236d2...`
- Raw content:
  ```text
  J6.5 R5 active non-message ~= 36.35 KB
  J6.6 R5 active non-message mean = 22,496 bytes
  current Standard mean = 21,685 bytes
  R5 reduction ~= 38.1%; residual fixed overhead vs Standard ~= 3.7%
  ```
- Interpretation: H-021定位的主固定成本已经按工具边界修复；active request只在顶层表达ordinary capability，TaskSpace control只表达Map生命周期。
- Time: 2026-07-12

## Evidence E-046: 本轮3次请求差来自standalone finish和测试未合批
- Related hypotheses:
  - H-021
- Direction: refines
- Type: request-path-reconstruction
- Source: R5-J6.6 paired rollout and native cadence report
- Prediction or plan link:
  - J6.6 live adoption observation
- Matched signal:
  - Standard 6 requests，R5 9 requests；R5有2次`finish_nodes`未带后续sibling action，pytest与validator也分别采样，`direct_tool_mixed_responses=0`。
- Correlation keys:
  - run `20260712-065907-459`
  - `finish_without_sibling_actions=2`
- Raw content:
  ```text
  Standard: discover | parallel reads | patch | pytest+validator | CLI | final
  R5: init+reads | reads | finish | patch | pytest | validator | CLI | finish | finish+end
  ```
- Interpretation: schema固定成本已明显收敛，但本轮请求放大仍由Agent未采用finish+sibling以及普通验证未合批构成。Runtime正确地没有拒绝、重写或自动补动作；该单样本不支持新增语义gate。
- Time: 2026-07-12

## Evidence E-047: Standard三次重复同样出现路径方差和瞬时低级错误
- Related hypotheses:
  - H-020
- Direction: supports
- Type: standard-control-parallel-reproduction
- Source: R5-J6.6 three left-only count-call-stack runs
- Prediction or plan link:
  - H-020 cross-mode variability prediction
- Matched signal:
  - 三次Standard均solved，请求为9/5/6、工具为15/9/6；run-1在已读README、pyproject和源码后仍先执行未设置`PYTHONPATH`的CLI，并短暂误判没有`__init__.py`，随后自行检查并纠正。
- Correlation keys:
  - `20260712-072154-807`
  - `20260712-072154-844`
  - `20260712-072154-791`
- Raw content:
  ```text
  run-1: python -m call_stack_counter
  output: /usr/local/bin/python: No module named call_stack_counter
  reasoning: there's no __init__.py or proper module structure
  later ls: src/__init__.py exists
  recovery: PYTHONPATH=src python -m call_stack_counter
  ```
- Interpretation: Standard也会在完整反馈可见时生成可避免的低级动作并自行恢复。三轮均无错误Patch上下文或不存在package路径，但1/3有环境命令错误；低频单样本错误不能仅按运行模式归因。
- Time: 2026-07-12

## Evidence E-048: 当前Input差距约九成来自额外Provider请求
- Related hypotheses:
  - H-021
- Direction: refines
- Type: request-level-token-counterfactual
- Source: R5-J6.6 count-call-stack paired final wire
- Prediction or plan link:
  - `docs/v0.0.5/build-R5/21-r5-input-token-optimization-audit.md`
- Matched signal:
  - R5比Standard多31,650 input tokens；按动作路径可移除的3次请求输入为27,640，按尾部请求计算为28,531，占87.3%-90.1%。
- Correlation keys:
  - run `20260712-065907-459`
- Raw content:
  ```text
  total delta = 75,316 - 43,666 = 31,650
  action-aligned removable = 8,740 + 9,196 + 9,704 = 27,640
  tail-request upper estimate = 9,338 + 9,489 + 9,704 = 28,531
  ```
- Interpretation: 当前最大Input优化不是裁剪语义，而是减少standalone finish和可合批验证造成的完整前缀重发。反事实区间只用于优先级，不作为精确因果账单。
- Time: 2026-07-12

## Evidence E-049: Map协议和projection仍存在机械重复
- Related hypotheses:
  - H-021
- Direction: supports
- Type: code-and-payload-structure-audit
- Source: J6.6 rollout, projection renderer and control outputs
- Prediction or plan link:
  - R5 Input token optimization P0/P1
- Matched signal:
  - 首轮activation为511 chars、blank projection为1,217 chars；init args为1,206 chars；finish args为262/262/551 chars。Projection同时输出recent event excerpt和同事件ref metadata，而原始tool history仍可见。
- Correlation keys:
  - `render_active_projection`
  - `projection_recent_tool_feedback`
  - `projection_result_refs_available`
- Raw content:
  ```text
  current_node_recent_events: event metadata + excerpt up to 1,200 chars each
  result_refs_available: repeats event id/node/kind/source/success/ref/lengths
  activation control_contract: runtime executes ... nested actions
  active J6.6 contract: finish_nodes followed by top-level sibling ordinary calls
  ```
- Interpretation: 下一压缩对象应是无损的字段去重、稀疏序列化和message/event ref复用。Activation中的`nested actions`已与J6.6协议不一致，既污染语义又可能降低sibling采用率，应优先修正。当前projection组件telemetry不可用，实施前先补精确bytes分账。
- Time: 2026-07-12

## Evidence E-050: 单轮projection为固定epoch base且control delta完整
- Related hypotheses:
  - H-022
- Direction: supports
- Type: final-wire-runtime-trace-code-path
- Source: R5-J6.6 paired right side
- Prediction or plan link:
  - H-022 fixed-hash and journal visibility clauses
- Matched signal:
  - 9次request的message index 2均为1,796 bytes、content hash均为`c30d837f...`；rollout仅有`trace-2`一个projection budget event，估算189 tokens；4个control call/output pair完整保留init和finish delta。
- Correlation keys:
  - run `20260712-065907-459`
  - `trace-2`
  - `c30d837f7c705d0903b7cdd09f3dcc554484939617eb834b4343bc55b1b10824`
- Raw content:
  ```text
  request 1..9: projection bytes=1796, same content hash
  projection_budget events: 1, projection_tokens=189
  session guard: if action_map_epoch_snapshot_present, do not append another projection
  raw journal: initialize_then_actions -> finish_nodes -> finish_nodes -> finish_then_end
  ```
- Interpretation: active-context uniqueness没有失败，但它保证的是每个payload只有一个epoch snapshot，不是每次都重建最新Map。语义变化由后置control journal保留，因此不应动态刷新前部projection；应压缩固定base并在新epoch/compaction时重建一次当前snapshot。`context-projection-summary`未提取已有trace，是独立观测缺口。
- Time: 2026-07-12

## Evidence E-051: Input结构重复已按机械字段边界收敛
- Related hypotheses:
  - H-022
- Direction: supports
- Type: implementation-and-regression
- Source: commits `99801e7`、`aecf410`、`7eaefc5`、`f713b35`、`e7783b5`
- Prediction or plan link:
  - `docs/v0.0.5/build-R5/21-r5-input-token-optimization-audit.md`
- Matched signal:
  - projection extractor可读取snapshot内嵌trace；blank base、Map写入schema、populated projection和success ack均删除重复字段，failure原文保持。
- Correlation keys:
  - `projection_budget`
  - `TaskSpaceControlBatchResultV1`
  - commits above
- Raw content:
  ```text
  codex-tools: 140 passed / 1 ignored
  core tools: 335 passed
  action-map: 12 passed
  TaskSpace scenarios: 7 passed
  cost/performance/harness selftests: PASS
  locked build: PASS
  ```
- Interpretation: 实施没有增加Runtime语义判断，也没有通过隐藏失败或普通工具反馈换取token下降。
- Time: 2026-07-12

## Evidence E-052: 固定base和control ack成本显著下降且缓存前缀保持
- Related hypotheses:
  - H-022
- Direction: supports
- Type: historical-structure-comparison
- Source: J6.6 historical run vs `20260712-084344-432`
- Prediction or plan link:
  - H-022 minimal epoch-base clause
- Matched signal:
  - activation + blank projection由1,796降到569 wire bytes；projection estimate由189降到70 tokens；active non-message由22,488降到22,107 bytes；one-step finish ack由242降到117 bytes。
- Correlation keys:
  - historical `20260712-065907-459`
  - current `20260712-084344-432`
- Raw content:
  ```text
  fixed system message: -68.3%
  projection estimate: -63.0%
  active fixed region: -381 bytes/request
  active-shape request 4+ cache hit: 97.08%
  J6.6 active-shape warm cache hit: 97.27%
  ```
- Interpretation: 结构瘦身取得直接收益，且没有破坏active append-only warm prefix。两轮Agent路径不同，不能把总token差当作精确before/after收益。
- Time: 2026-07-12

## Evidence E-053: 当前control错误不是反馈丢失
- Related hypotheses:
  - H-020
  - H-022
- Direction: refutes-context-loss
- Type: live-rollout-trace
- Source: current R5 Docker paired run
- Prediction or plan link:
  - feedback visibility clause
- Matched signal:
  - epoch base明确标为历史snapshot；init调用和成功输出完整保留；输出包含3个Agent节点ID。Agent读完源码后仍尝试create-and-bind新node，被`current_main_node_running`忠实拒绝，下一请求自行改用`finish_nodes`。
- Correlation keys:
  - `call_00_vw0xuByhIwfkv4qUwDgU8735`
  - `call_00_hTCAmDRsCU7OYemZfuyZ8630`
  - `call_00_Y6ZOtIi9SeAs32R3t8Fx1284`
- Raw content:
  ```text
  init output: current_node_id=explore-codebase; node_ids=[explore-codebase, implement-fix, verify-fix]
  rejected action: create_node(bind_current=true)
  hard reason: current_main_node_running
  recovery: finish_nodes(explore-codebase -> implement-fix)
  ```
- Interpretation: 本次错误动作发生在完整状态反馈可见的情况下，属于Agent路径方差；Runtime硬拒绝和反馈语义正确。单样本不支持增加Runtime语义约束。
- Time: 2026-07-12

## Hypothesis H-023: nested action 未进入 canonical Event Store 导致工具 reservation 永久泄漏
- Status: confirmed-fixed
- Parent: P-001
- Claim: J6.7.5 将 Map tool result 改为必须引用 canonical event 后，`initialize_then_actions` 内的 nested ordinary call 没有独立写入 Event Store；result attribution 因找不到 call event 而在释放 reservation 前失败，留下永久 `node_tool_calls_in_flight`。
- Layer: canonical-context-and-tool-lifecycle
- Factor relation: causal
- Depends on:
  - H-017
- Rationale:
  - nested action 仍以 outer control arguments/output 承载，违反 J6.7 的独立原生 call/output pair 合同；只有 nested call 缺 canonical ID 才能同时解释“工具已经完成”“Map没有对应 node event”和“in-flight计数永不归零”。
- Falsifiable predictions:
  - If true: outer init 的两个 nested call 有真实成功输出，但 `task_context_event_recorded` 中没有对应 nested call/output；Map snapshot 缺这两个 node event；首次 finish 精确报2个in-flight。
  - If false: Event Store 已存在两个 nested call event，或 reservation 在 result path 中已释放，或in-flight数量来自尚未结束的真实进程。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对 outer init aggregate、canonical event序列、Map node events和首次finish错误做call_id逐项join。
  - Signal: `${outer}:nested:0/1`、`task-event-*`、`node-event-*`、`node_tool_calls_in_flight`。
  - Capture method: interrupted Docker rollout与最终Map snapshot；静态检查 `execute_taskspace_barrier -> record_main_tool_result_with_class -> event_id_for_call`。
  - Event name or marker:
    - `task_context_event_recorded`
    - `node_event_recorded`
    - `node_tool_calls_in_flight`
  - Correlation keys:
    - run `20260712-122002-493`
    - outer call `call_00_etl6GnuK6M0jrUjjAwuC0653`
  - Differentiates from:
    - Agent忽略finish说明。
    - final gate反馈丢失。
    - 普通并行工具尚未完成。
  - Supports if:
    - nested call/output只存在于outer正文，且in-flight恒等于缺失canonical pair数量。
  - Refutes if:
    - nested pair已独立持久化或reservation能在无canonical event时正常完成。
  - Instrumentation status: existing-runtime-evidence
  - Instrumentation lifecycle:
    - 保留 canonical event、node event 与 hard-state error 的关联字段。
- Evidence gate: satisfied
- Related evidence:
  - E-054
  - E-055
- Conclusion: confirmed；修复必须补齐nested原生事件入口并消除outer正文重复，不得弱化in-flight硬规则或让Runtime猜测工具已完成
- Repair design readiness: ready
- Next step: 先写nested event/reservation回归，再实现call/output独立记录与outer event-ref aggregate。
- Blocker:
  - none
- Close reason:
  - nested pair ingress、reservation closure与同样本Docker复验均通过

## Evidence E-054: 缺失的两个nested canonical pair与永久in-flight数量精确相等
- Related hypotheses:
  - H-023
- Direction: supports
- Type: live-rollout-and-code-path
- Source: J6.7.5 `count-call-stack` interrupted R5 Docker run
- Prediction or plan link:
  - H-023 canonical pair join clause
- Matched signal:
  - outer init output明确包含两个成功nested shell output；canonical sequence只有outer call/output，首个独立普通call从`task-event-8`开始；Map最终有13个后续tool events但没有两个init nested events；首次及所有后续finish均稳定报`2 in-flight main tool call(s)`。
- Correlation keys:
  - run `20260712-122002-493`
  - `call_00_etl6GnuK6M0jrUjjAwuC0653:nested:0`
  - `call_00_etl6GnuK6M0jrUjjAwuC0653:nested:1`
  - first finish `call_00_GuqWSnMiwRycNh5lRnUn4840`
- Raw content:
  ```text
  task-event-5/6: outer taskspace_control call/output
  nested:0/1: only embedded inside outer output, both success=true
  first finish: node understand_project has 2 in-flight main tool call(s)
  record_main_tool_result_with_class: requires source_event_id before release_main_tool_reservation
  ```
- Interpretation: 硬规则和错误反馈是正确的；错误状态来自canonical工具事件缺位。通过放宽finish、自动清reservation或让Agent继续重试都会掩盖事实链缺口。
- Time: 2026-07-12

## Evidence E-055: nested pair物化后同样本完整闭合且无orphan
- Related hypotheses:
  - H-023
- Direction: fix-validation
- Type: unit-integration-and-live-docker
- Source: commits pending + J6.7.5 `count-call-stack` rerun
- Prediction or plan link:
  - H-023 fix criteria 1-5
- Matched signal:
  - 3个nested action分别形成3组独立call/output event，均携带outer `parent_call_id`和`node=explore` owner；outer output只含event refs；run在10 requests内结束，task/map与4个node均completed，open/orphan call/output均为0。
- Correlation keys:
  - run `20260712-123635-091`
  - nested events `task-event-6..11`
- Raw content:
  ```text
  Standard: solved, 8 requests, 16.51s
  R5: solved, 10 requests, 24.52s
  R5 nested actions: 3
  orphan calls/outputs: 0/0
  root task: completed
  open nodes: 0
  action-map tests: 28 passed
  tools tests: 341 passed
  ```
- Interpretation: 原始95-request runaway不再复现；修复补齐了canonical工具链路，没有放宽hard state、自动finish或添加Runtime语义决策。
- Time: 2026-07-12
