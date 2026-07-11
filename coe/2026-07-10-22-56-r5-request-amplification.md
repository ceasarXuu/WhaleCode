# Problem P-001: R5 TaskSpace 在 G1 正确性样本中请求次数显著高于 Standard
- Status: repairing-j6-schema-reference-gap
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
- Status: confirmed
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
- Conclusion: confirmed
- Repair design readiness: ready and authorized by the active J6 implementation request
- Next step: 初始化 schema 直接接收 Agent-authored node id，依赖、current binding 和后续 finish 全程使用同一标识；不保留 key/id 双轨兼容。
- Blocker:
  - none
- Close reason:
  - not closed

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
