# R5 Native Tool 状态屏障与终态收敛计划

## 1. 元数据

- Created: 2026-07-10
- Updated: 2026-07-12
- Version: v0.0.5 build-R5 follow-up
- Status: J0-J5 engineering complete; live chained-finish adoption remains unproven; J6 redesign planned
- Owner / Responsible: WhaleCode core runtime
- Related Systems: provider tool choice、native tool scheduler、TaskSpace hard state、taskspace_control、turn completion、benchmark observer
- Related Links: `01-r5-phased-simplification-plan.md`、`13-r5-unified-docker-benchmark-and-logging-plan.md`、`17-r5-schema-first-taskspace-control-plan.md`、`coe/2026-07-10-22-56-r5-request-amplification.md`
- Risk Level: High
- Plan Type: Full

## 2. 问题定义

R5 G1 三轮 `count-call-stack` 中，Standard 为22个 provider requests，R5 为51个。29次差值已闭合为12个 Map control 和17个额外普通工具调用。

进一步审计确认：

1. 三轮首请求都完整包含 `active_task_path_without_nodes` 和 `taskspace_control(action=initialize_map)`；两轮仍先调用普通工具，不是提示或 projection 丢失。
2. provider tool surface 在机械空 Map 时仍暴露普通工具，能力可见性与现有 hard state 不一致。
3. `finish_node` 已原子完成当前节点并绑定下一节点，三轮没有额外 `bind_node`。
4. 当前 native scheduler 把同一 response 的 calls 放入 in-flight 执行；普通工具的 TaskSpace preflight 位于 execution lock 之前，不能安全表达 `state transition -> dependent tool`。
5. 最后一个 `finish_node` 成功后还需要一次 provider request 才能生成最终回答。

本计划只降低机械协议往返，不通过减少节点、自动完成节点、语义提示或 runtime 决策来追求 request 数字。

## 3. 决策与边界

纳入实施：

- **P0**：让 provider 工具选择与机械 hard state 对齐。
- **P1**：为 native tools 增加 Agent-authored、runtime-mechanical 的有序状态屏障。
- **P3**：允许 Agent 把最后节点完成和自己生成的 final candidate 作为一个终态事务提交。

明确拒绝：

- **P2**：不以更粗 Map、减少节点、自动合并节点或限制节点数量作为降本方案。

P2 有 Map 退化坍缩风险，会让复杂任务重新回到少量节点吸收大量不相干工作。所有 J 阶段收益必须在固定 Map 拓扑或拓扑健康不下降的前提下成立；节点减少不能计为收益。

## 4. 设计原则

1. Agent 决定 Map、节点、动作、顺序和最终回答内容。
2. runtime 只执行 Agent 明确提交的动作，维护权限、沙箱、状态机和失败停止等硬规则。
3. runtime 不从工具输出推断“应当 finish”，不自动选择下一节点，不生成 final 文本。
4. 每个工具继续使用原工具 handler、权限、日志、call id 和原始反馈；不得包成丢失细节的语义摘要。
5. 有依赖的步骤严格顺序执行；无依赖且声明可并行的普通工具继续并行。
6. 旧 `taskspace-action-sequence-v1` 只能作为机械顺序执行的历史证据，不恢复其禁用 native tools 的 transport、JSON envelope 或提示层。
7. 不保留运行时兼容模式或双路径；phase 失败通过 git revert 回退。

## 5. 外部依据

1. [DeepSeek Tool Calls](https://api-docs.deepseek.com/guides/tool_calls)：DeepSeek 使用标准 tools/function call 流程，工具定义和执行结果需由客户端管理；本计划继续使用 native tool loop。
2. [OpenAI Chat Completions API](https://platform.openai.com/docs/api-reference/chat/completions/create)：`tool_choice` 可指定函数，`parallel_tool_calls` 表达并行工具能力；P0 使用前必须在 DeepSeek 实际 provider probe 中验证兼容字段，P1 不把并行调用误当依赖序列。
3. [Tokio RwLock](https://docs.rs/tokio/latest/tokio/sync/struct.RwLock.html)：锁只保证申请后的互斥和公平性，不替代业务状态 preflight 的顺序事务；TaskSpace preflight 必须进入显式 barrier 执行链。

以上依据不构成“DeepSeek 原生 tool loop 不稳定”的假设。计划只验证明确的 API 能力和 Whale 当前调度顺序。

## 6. 目标与非目标

### 6.1 目标

| Goal | Expected Benefit | Verification |
|---|---|---|
| 消除空 Map 无效普通工具尝试 | 不再用一次 reject + retry 才进入初始化 | pre-init ordinary call=0；首请求成功 initialize coverage=100% |
| 合并状态迁移与已知后续动作 | 减少 control-only response 和 provider resampling | 固定三节点 fixture 中 init/read、finish/edit、finish/test 可同 response 完成 |
| 收敛最后 finish 往返 | 最后节点完成后无需再次采样相同上下文只为生成 final | terminal transaction 成功时 extra final request=0 |
| 保持工具语义和安全边界 | 降本不牺牲反馈、权限或错误语义 | 每步原始 call/output 可回放；permissions/sandbox tests 通过 |
| 防止 Map 坍缩 | 请求下降不以图结构退化换取 | 固定 topology benefit gate；复杂样本 map health 不下降 |

### 6.2 非目标

- 不让 runtime 自动初始化 Agent 语义 Map。
- 不提示 Agent “少建节点”“合并阶段”或指定任务策略。
- 不把 read/edit/test 的成功解释为节点自动完成。
- 不恢复旧 action-contract transport。
- 不改变 Standard 的工具语义；通用 scheduler 改动必须保持 Standard 行为。
- 不在 Docker I2 完成前用宿主机样本声明正式成本收益。

## 7. 总体设计

```text
provider response
  -> ordered response items
  -> parallel ordinary segment (optional)
  -> state barrier: taskspace_control
       execute atomically
       capture exact output
       refresh hard state / binding / lease
  -> dependent segment
       preflight against latest state
       execute with original handler and permissions
  -> stop and mark later calls skipped after first barrier/protocol/tool failure
```

P0 只调整机械空 Map 时的 provider tool selection，不改变 Map 语义。P1 把 state transition 变成 native scheduler barrier，不创造 TaskSpace 私有工具执行器。P3 只转发 Agent 已生成的 final candidate，不允许 runtime 编写或改写回答。

## 8. 分阶段执行计划

### R5-J0：契约冻结与固定拓扑基线

**Entry:** R5-F 100%完成；R5-I2 已提供统一 Docker paired runner。

**Tasks:**

1. 冻结三节点 deterministic fixture：inspect -> implement -> validate；所有收益测试保持相同 node/edge/current-node 序列。
2. 冻结 provider payload 中 tools、tool choice、parallel flag 和 response item order 的 trace schema。
3. 对 DeepSeek 实际 ChatCompletions 执行 capability probe：named `tool_choice`、多 tool calls 顺序、assistant content + tool calls 可用性。
4. 冻结 barrier failure contract：首个失败后，后续 call 不执行，但每个 call id 都得到明确 `skipped_due_to_prior_failure` 机械反馈。
5. 冻结 terminal candidate provenance：必须标记为 Agent 原文，runtime 不可生成、摘要或修订。

**Exit:** provider probe、固定拓扑 fixture、失败矩阵和日志 schema 均有独立 artifact；不依赖 J1 实现证明。

**实施结果（2026-07-11）：** J0 已完成。真实 DeepSeek probe 证明：named
`tool_choice=taskspace_control` 在 `thinking=disabled` 时返回 HTTP 200 且只选择目标工具；thinking
开启时 provider 返回 HTTP 400；`required + parallel_tool_calls=true`
可在同一响应按 `first_step -> second_step` 顺序返回两个 calls；`assistant content + required tool`
未观察到 content。因此 J1 冻结为 named choice + mechanical thinking disabled，J2 只认 provider
response item order，J3 冻结为 `finish_node.final_candidate` 参数载体。

固定三节点拓扑、首错停止、`taskspace-skipped-tool-output-v1` 和终态 provenance 已写入
`benchmarks/taskspace/native-control-contract.json`；可复跑 probe 位于
`scripts/taskspace-benchmark/probe-native-control-provider.ps1`，本次 artifact 位于
`target/r5-j0-provider-probe/provider-capability.json`。诊断只记录状态、工具名、长度和 hash，
不记录候选正文或 API key。

**Fallback:** capability 不满足时暂停对应子项并记录事实，不改成语义 prompt 或旧 action envelope。

### R5-J1：P0 hard-state-aware tool selection

**Entry:** J0 100%完成。

**Tasks:**

1. 在 `active_task_path_without_nodes` 时优先使用经 J0 验证的 named `tool_choice=taskspace_control`，保持 tools schema/hash 不变。
2. 只从已有 hard state 产生 tool-choice 决策；不得读取用户文本、节点标题或任务内容。
3. initialize 成功后恢复普通 provider tool selection。
4. 如果 DeepSeek 不接受 named tool choice，只允许采用 J0 验证过的 hard-state visibility narrowing；该选择必须单路径落地并重新通过 cache/tools-hash 门禁。
5. 记录 tool choice 的状态来源、provider payload 和拒绝原因，不记录用户正文。

**Exit:** 空 Map focused test 中首响应 ordinary tool=0、initialize success=100%；非空 Map 不强制 control；G1 strict-prefix/cache gate 不回退；无新增初始化提示。

**实施结果（2026-07-11）：** J1 已完成。`Prompt.tool_choice` 已改为 typed provider
contract；机械空 Map 时首请求使用 named `taskspace_control`，初始化成功后恢复 `auto`，tools
集合和 schema 不变。DeepSeek 在 named choice 时按 J0 contract 关闭 thinking；该响应产生的 assistant
tool-call history 从首次出现起固定携带空 `reasoning_content` 字段，忠实表达“无 reasoning”，避免后续
thinking 恢复时 provider 拒绝历史消息，同时不改写任何已有 reasoning。

focused tests 覆盖空 Map 选择、初始化后释放、Chat body 和稳定历史字段。Docker 实跑
`target/r5-j1-hard-state-selection-final/count-call-stack/20260711-042317-066` 证明：首请求只选择
`taskspace_control`，普通工具调用前已完成初始化；19次请求的 tools hash 全部相同，后续18次 wire
prefix 比较18/18保持，cache hit 97.68%，业务成功且无 provider protocol error。该样本19次请求和9次
control 调用不计为 J1 收益，留给 J2/J3 收敛。

**Rollback:** revert J1 commit；不保留 named-choice/visibility 双分支。

### R5-J2：P1 native ordered state barrier

**Entry:** J1 100%完成。

**Tasks:**

1. 在通用 native tool scheduler 中引入 response-local ordered segments；`taskspace_control` 作为状态 barrier。
2. 将普通工具 TaskSpace preflight 移入实际执行序列，在前置 barrier 成功后读取最新 binding/lease/status。
3. barrier 前的可并行普通工具先完成；barrier 自身独占；barrier 后的调用不得提前 preflight、预约或归属旧节点。
4. 任一 barrier、协议、权限或工具失败后停止 dependent tail；保留已执行输出，并为未执行 call id 产生机械 skipped output。
5. 保留每个工具原 handler、审批、沙箱、diff tracker、call/output history、NodeEvent 和 telemetry。
6. 工具说明只描述“同 response 按声明顺序执行”的机械能力，不建议具体任务动作或节点策略。

**Exit:** 以下固定拓扑序列均通过正反测试：`initialize -> read`、`finish inspect -> edit`、`finish implement -> test`；失败 barrier 后无 tail side effect；普通并行工具性能不回退；节点 attribution 100%指向最新 binding。

**实施结果（2026-07-11）：** J2 已完成。provider response item 先按原顺序收集为原始
`ToolCall`，通用 scheduler 再拆为普通并行段和单个 `taskspace_control` 状态屏障。没有动作重排、自动
插入或参数解释；每个调用仍进入原 router/handler/permission/sandbox/diff tracker，TaskSpace preflight
只因执行被延后而自然读取到屏障后的最新状态。普通工具-only response 保持一个并行段。

失败停止使用 `TaskSpaceToolSkippedV1` 机械输出，保留未执行工具的原 call id，并只记录
`skipped_due_to_prior_failure` 和前置失败 call id。日志增加 response sequence、parallel segment、barrier
started/completed/failed 和 skipped call 事件，不记录工具参数正文。真正失败或取消的不完整 provider
response 不执行未调度副作用；mailbox 正常抢占则在统一收尾点执行已完整收到的调用。

unit tests 覆盖 provider 顺序分段、ordinary-only 并行段和 skipped output。真实 session 集成测试使用固定
三节点 Map，在同一 response 内分别通过 `initialize -> read`、`finish inspect -> edit`、`finish implement
-> test`，最终3个节点均完成；反向测试让 bind barrier 失败，确认 dependent shell 文件副作用为零且
下一请求同时包含失败和 skipped output。Standard 回归证明 MCP opt-in 并发和默认串行语义均未改变；
测试前需先执行 `cargo build -p codex-rmcp-client --bin test_stdio_server --locked`，否则 fixture 缺失会在
业务断言前失败。

**Rollback:** revert J2 commit，回到每个 control 独立 round trip；不恢复 action-contract transport。

### R5-J3：P3 Agent-authored terminal transaction

**Entry:** J2 100%完成；J0 已确认 terminal carrier。

**Tasks:**

1. 允许 Agent 在最后节点提交 `finish_node` 和自己的 final candidate，二者组成显式终态事务。
2. runtime 先执行 finish 和既有 final hard gate；全部成功后按原字节发布 Agent candidate。
3. finish/gate 失败时不发布 candidate，返回原始失败并继续 native tool loop。
4. conversation history 必须保留 finish call/output 和 Agent final message 的可回放顺序；不能把 final 标成 runtime-generated。
5. 禁止 runtime 从 `result_summary` 生成 final，禁止固定模板或 fallback 回答。

**Exit:** 成功路径不产生 finish 后额外 provider request；失败路径不泄漏 candidate、不伪装 Agent completion；自然语言 final 100%可追溯到模型输出；Standard completion path 不变。

**实施结果（2026-07-11）：** J3 已完成。`finish_node` 增加可选 `final_candidate`，且不能与
next-node 参数组合。ActionMap runtime 使用 clone-and-commit 对最后节点 finish 和既有 final lifecycle
gate 做原子校验；候选为空、仍有 current/runnable 节点或状态转换失败时不提交 staged state。final gate
补齐了既有硬状态缺口：除 current node 外，Map 中仍有 ready/running 节点时也不能结束；blocked 导致的
pending 分支仍可由 Agent报告阻塞结果。

候选通过 `ToolOutput -> AnyToolResult -> ToolCallExecution -> ToolSequenceOutcome` typed metadata 传递，
turn 层只按原字节生成 Agent `final_answer` history item，不解析工具输出、不从 result summary 生成回答。
候选发布后同 response 尾部调用得到 `skipped_due_to_terminal_completion` 机械反馈。日志只记录 call id 和
字节数；`taskspace_control` diagnostic payload 中候选正文被结构化脱敏，provider history 保持原始参数。

unit tests 覆盖 schema、解析、日志脱敏、原子成功和原子拒绝。真实 session 集成 fixture 仅产生2次
provider requests：第一轮 initialize/read，第二轮 `finish_node(final_candidate)`，随后直接 TurnComplete；
不存在第三次 final request，Map 单节点 completed，rollout 中 Agent 原文以 `phase=final_answer` 保存。
相关 tools/core focused 与三条 ordered/failed/terminal session scenarios 全部通过。全量 core 在加载
仓库 `.env.local` 后为1773 passed、2 failed、3 ignored；仅剩两条未改动 file-watcher mock 注册状态测试
失败，J3 调用链不引用该模块，作为 J4 全局对抗审查的测试基线异常单独复核，不用于掩盖 J3 门禁。

**Rollback:** revert J3 commit，保留 J1/J2；不维护双终态协议。

### R5-J4：固定拓扑收益与对抗性门禁

**Entry:** J3 100%完成。

**Tasks:**

1. Docker 下执行 `count-call-stack` Standard/R5 交替三轮，固定三节点拓扑单独报告机制成本。
2. 执行一个复杂依赖样本，验证节点/边/结果归属、并行能力和 final lifecycle。
3. 对抗性测试错误顺序、失败 barrier、权限拒绝、取消、timeout、重复 call id 和 malformed terminal candidate。
4. 使用 performance observer 报告 request、tool-bearing response、control-only response、barrier batch、tokens、cache、wall 和 Map topology。
5. 独立审计 provider-visible history，确认没有新增策略提示、语义摘要或旧 action-contract 文案。

**Exit:** 固定拓扑下 R5 control-only response 从4/轮降到不高于1/轮；pre-init reject=0；terminal extra request=0；correctness/cache/feedback/map health 全部通过；复杂样本无 Map 坍缩。

**实施结果（2026-07-11）：J4 correctness/capability 验证完成，收益门禁未通过。**

- `count-call-stack` 三轮 Docker paired run：Standard/R5 均 `3/3 solved`，prefix coverage 均100%，
  R5 Map 分别为5/6/3节点；但 R5 总请求54 vs Standard 22，control-only 分别12/13/7，
  mixed barrier 均为0。证据：`target/r5-j4-clean/count-call-stack/20260711-054857-084`。
- 修复 Standard 错误暴露 `taskspace_control` 后，post-fix paired run 两侧 solved，Standard control=0；
  R5 仍为7个 control-only、0 mixed barrier。证据：
  `target/r5-j4-batching-contract/count-call-stack/20260711-060333-715`。
- `multi-file-order-pipeline` 两侧 solved；R5 为6节点/6边，未坍缩，tools 14 vs 15、wall 1.10x，
  但 requests 22 vs 8、control-only=13、mixed=0。
- `subscription-billing-repair` 两侧 solved；R5 为8节点/11边，requests 29 vs 20、wall 1.38x，
  control-only=18、mixed=0。
- Agent 提交 `final_candidate` 时 terminal extra request=0；真实 run 中 Agent 未选择该可选字段时仍为1。
  runtime 没有自动生成、合并或强迫终态。
- barrier 顺序/失败停止与 terminal transaction 的 production path、focused tests 和真实日志均存在，
  但真实 DeepSeek run 没有产出混合 barrier batch。只补充通用 barrier batching 说明后，单样本仍为0 mixed。

进一步按 G1 历史行为、J4 call/output 和工具 schema 提交历史对账后，原结论需要拆成两部分：

1. **原子状态动作发生了工具契约回归。** G1 三轮都使用
   `finish_node(next_node_id)`，每轮 `bind_node=0`。`d2cc4b7` 删除过度设计时，也删掉了
   `current_node_key` 会建立立即可用 binding、`node_id` 默认当前 binding、`finish_node` 可原子绑定下一节点
   的机械说明。J4 因此稳定退化为 `initialize + N*bind + N*finish`。初始化工具输出本身完整返回
   `current_node=node-1`，这里是工具能力契约的语义缺失，不是输出丢失或扭曲。
2. **跨工具 mixed barrier 是另一类问题。** DeepSeek/provider 支持多工具，Standard 和 R5 也继续批量
   提交独立 reads/tests；但模型在生成同一响应的所有 calls 时看不到前一个工具的执行结果。状态迁移后的
   ordinary action 依赖 finish/barrier 成功，Agent 选择等待结果再请求是合理的保守行为。J2 只保证 runtime
   能安全执行 Agent 已预声明的顺序，不会也不应强迫 Agent 预声明。

因此后续收益门禁修正为：先恢复简洁、真实、不含任务策略的机械 API 契约，并验证固定拓扑
`bind_node=0`、control 从 `2N+1` 回到 `N+1`。`mixed barrier` 继续作为能力和行为观测项，不再把
`control-only <= 1/run` 当成当前原生 tool loop 下必须达到的硬收益指标。禁止用 runtime 自动绑定、自动
合并、拒绝合法重复 bind 或提示任务策略来替代契约修复。

**机械契约修复与单轮复验（2026-07-11）：** `f0db9d7` 已在 tool description 和字段 schema 中恢复
`initialize_map` 同步绑定 `current_node_key`、`finish_node.node_id` 默认当前 binding、
`finish_node.next_node_id` 原子完成并绑定下一节点等真实机械效果。字段级 schema tests、registry test 和
Whale build 均通过；fix-validation 的 tools hash 相对旧 run 已变化，并在21次请求中保持21/21一致，确认
新契约实际送达 provider。

Docker paired run `target/r5-j4-mechanical-contract/count-call-stack/20260711-181112-154` 双侧 solved，
Standard 为7 requests/13 tools/15.04s，TaskSpace 为21 requests/13 ordinary tools/12 controls/43.47s。
TaskSpace 创建5节点4边，全部节点完成；terminal candidate 生效，extra final request=0；request-2+ cache
hit 为96.33%，prefix 20/20。行为收益没有成立：`next_node_id` 使用仍为0，成功状态推进仍有4次独立
bind，另有2次失败 control。

原始链路进一步表明，Agent 已知道初始化自带 binding，也明确复述了 finish 可默认当前 binding；但它把
初始化输出 `node_ids=[read-readme-and-tests=node-1,...]` 读成
`node-1=read-readme-and-tests`，随后把 node key 当作 node id 调用 finish/bind，各触发一次原样 hard
error。由此，契约暴露缺失已修，但它不是独立 bind 的充分根因；新增直接问题是初始化反馈映射方向不够
无歧义。性能表中的 `Failed=0` 只统计 ordinary tool failure，当前没有单列这2次 control hard error，后续
observer 需要补充 control failure 计数。下一步应先将初始化结果改成方向显式的结构化机械数据，再单独
验证 `next_node_id` 采用情况。

本轮第一次运行被 binary health 在 Agent 启动前正确阻止：commit 时间晚于 binary mtime 且旧
attestation 不匹配。标准恢复流程是重新执行 locked build，并调用
`write-whale-binary-attestation.ps1` 记录当前 commit 和 binary SHA；不得使用 stale bypass 产生正式样本。

**显式映射修复与第二轮复验（2026-07-11）：** `6c0153c` 删除初始化结果中的手工 `key=id` 文本，
改为 `TaskSpaceInitializeMapResultV1` JSON，独立提供 `current_node_key`、`current_node_id` 和
`node_id_by_key`。结果只保留 runtime 已提交的机械事实，不含动作建议或任务策略。生产 handler 的测试
被拆到独立文件，主文件从527行降到476行；8个 focused core tests、locked build 和 binary attestation
均通过。

Docker paired run `target/r5-j4-explicit-init-mapping/count-call-stack/20260711-183707-628` 再次双侧
solved。TaskSpace 的4节点3边全部完成，node key/id hard error 从2降到0，`bind_node` 从上一轮5降到0；
Agent 连续提交 `node-1 -> node-2 -> node-3 -> node-4` 的3次 `finish_node(next_node_id)`，最终 controls
为 `initialize_map=1 + finish_node=4 = N+1`。terminal candidate 正常，extra final request=0，mixed
barrier 仍为0。

同轮 Standard/R5 分别为5/12 requests、10/13 ordinary tools、12.35s/24.61s、36,393/96,949 input
tokens，request-2+ cache hit 为92.71%/93.43%。相较上一轮非同拓扑 R5，requests 21 -> 12、controls
12 -> 5、wall 43.47s -> 24.61s、input 194,687 -> 96,949；由于 Map 从5节点变4节点且模型采样不同，
总量下降不能全部归因，但错误归零、bind归零和原子 next 恢复由 call/output 链直接证明。剩余问题已经
收敛为必要 `N+1` control-only 往返、3个额外 ordinary actions，以及 mixed barrier 未采用。

因此 J4 的 `control-only <= 1/run` 明确未达成，Decision 为 **hold benefit claim**。不通过自动绑定、
强制 `next_node_id`、拒绝合法重复 bind 或 runtime 合并动作来追指标；这些方向会越过 Agent 对 Map
推进的所有权。后续若继续优化，应从 Agent 可理解的状态事实与工具能力使用效率入手，并保持可选性。

**Fallback:** 任何 correctness、反馈、权限或 Map health 回退都阻止收益声明，并按 J3 -> J2 -> J1 的独立 commit 逆序回退。

### R5-J5：Agent-authored chained finish cadence

**Entry:** J4 显式映射复验已达到 `bind_node=0` 和 `N+1 controls`，但5个 control 全部独占 response，
真实 mixed barrier 仍为0。

**机械契约：**

1. 一个 provider response 可以声明多个 `taskspace_control`，也可以交错声明 control 和 ordinary calls。
2. 多个 finish 不是并发写状态；provider 一次声明，runtime 按 response order 串行提交，每一步读取前一步
   更新后的 binding/lease/status。
3. 后续调用只在前置 barrier 成功时执行；失败后保持原始失败输出，并给剩余 call id 返回
   `skipped_due_to_prior_failure`。
4. 带非空 `final_candidate` 的 finish 是 terminal finish，可以位于响应末尾。
5. 不带 `final_candidate` 的 finish 是 nonterminal finish。工具说明和激活上下文鼓励在同一响应继续
   Agent 已知的 control 或 ordinary call，但 standalone finish 保持合法；runtime 只记录 cadence observation，
   不拒绝、不选择或补写下一动作。
6. `finish A -> finish B` 与 `finish A -> ordinary action on B` 都是允许的 Agent-authored sequence。
7. 当前没有 binding 时，带显式 `node_id` 的 finish 可在同一事务中 claim ready target 并完成；pending、
   completed、租约冲突和当前其他节点仍运行等硬状态继续拒绝。

**Tasks:**

1. 在 tool description 中明确 multiple controls、latest-state execution、首错停止和 trailing finish 规则。
2. provider probe 增加同一 function name 重复调用，区分“多工具可用”与“重复 control 可用”。
3. native sequence executor 对 trailing nonterminal finish 只写结构化 observation，不改变工具成功语义。
4. focused tests 覆盖相邻 finish barriers、finish+ordinary、前错跳过、standalone allow、显式 target
   原子 claim、失败无副作用和 terminal allow。
5. Docker `count-call-stack` 复验 control-only、mixed barrier、request、correctness、cache 和 Map health。

**Exit:** provider repeated-function probe通过；standalone nonterminal finish正常提交；显式 ready target可在
无 binding 时原子完成且失败无副作用；terminal finish不被误拒。真实 chained finish / finish+ordinary采用率
作为独立行为收益门禁，未观察到时不得把工程能力完成写成 cadence收益完成。

**边界：** 不允许 runtime 自动 finish、自动选择 next node、自动生成 ordinary call、合并 Agent 未声明的
节点或从工具结果推断任务完成。cadence 是引导和观测，不是状态机正确性 gate。

**实施与验证结果（2026-07-11）：**

1. Provider probe `ordered_repeated_control_calls` 返回 HTTP 200，并按
   `taskspace_control(finish_first) -> taskspace_control(finish_second)` 返回两个同名 calls。
2. hard gate 三轮 R5 分别为17/15/17 requests，cadence rejects为2/2/6。最强引导轮中 Agent在6次拒绝后
   用3个无意义 `echo "follow-up after finishing node-X"` 满足形式规则，证明 hard ban制造纠错请求和上下文
   污染，已由 `e3261aa` 删除。
3. advisory run `20260711-201839-033` 两侧 solved，no-op和 cadence reject归零；但 R5 仍为19 requests、
   12 controls、3 control failures、3 bind。调用链证明 standalone finish释放 binding 后，显式
   `finish_node(node_id=node-3)` 被 `no_current_node_binding` 拒绝。
4. `d0f35ca` 让显式 ready target在无 binding时原子 claim+finish；pending target反例验证失败不留下 binding
   或 lease。同一 mock provider response内相邻两个 finish只需两次总 provider requests完成初始化和终态。
5. 最终 Docker run `20260711-203035-327` 两侧 solved。R5 为10 requests、12 ordinary tools、5 controls、
   0 control failures、0 bind、4节点全部完成、terminal extra request=0；Standard为8 requests、13 tools。
   但 R5 的5个 controls仍对应5个 control-only responses，`multi-control=0`、`chained-finish=0`、
   `mixed barrier=0`。Map 为4节点0边，不能把相对上一轮的总量下降全部归因于修复。

**Decision:** J5 工具与执行能力、反馈语义、硬边界和观测建设完成；hard gate方案被否决并回撤。真实 Agent
尚未稳定采用同响应多 finish 或 finish+ordinary，因此 cadence行为收益保持 **hold**，不继续通过 runtime
约束或语义注入追指标。

**J6 follow-up（2026-07-12）：** 后续不再把“同一 response 追加兄弟 tool call”作为
`taskspace_control` description 或 runtime cadence gate。根因是当前 function schema 只描述单个
`finish_node`，无法约束兄弟调用；schema、提示词和 runtime 因此互相矛盾。J6 将范围限定为演进现有
`taskspace_control`：用 schema 内 required actions 表达 `initialize + actions`、`finish + actions` 和
`finish + end`，内部 ordinary actions 机械复用现有 ToolRouter。不得新增公开 action-frame tool，不得恢复
后置 cadence reject，不得由 runtime 选择下一动作。详细设计和 phase gate 见
`17-r5-schema-first-taskspace-control-plan.md`。

## 9. Phase Gate Matrix

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Required Before Next | Decision |
|---|---|---|---|---|---|
| J0 | provider probe、fixed-topology fixture、failure contract | 不依赖 J1 code | capability/contract artifacts | 100% passed | proceed J1 |
| J1 | hard-state tool-choice fixtures、wire/cache trace | 不依赖 barrier | first-response and tools-hash evidence | 100% passed | proceed J2 |
| J2 | ordered barrier unit/integration/side-effect tests | 不依赖 terminal transaction | latest-state attribution and stop evidence | 100% passed | proceed J3 |
| J3 | terminal success/rejection/history tests | 不依赖 benefit repeats | completion provenance evidence | 100% passed | proceed J4 |
| J4 | Docker fixed-topology repeats、complex sample、adversarial review | 无后续 phase 补证 | performance/map/semantic report | correctness complete; mapping repair validated; N+1 control-only remains | hold remaining cadence benefit |
| J5 | repeated-control probe、advisory cadence、atomic explicit finish、Docker live run | 无后续 phase 补证 | sequence/integration tests、control failure与 multi-control trace | engineering gates passed; live adoption absent | capability complete; behavior benefit hold |
| J6 | schema/provider probe、discriminated tool contract、native router reuse、Docker paired samples | 无后续 phase 补证 | standalone finish不可表示；忠实 batch output；control-only边界归零 | correctness + behavior benefit 100% | planned |

## 10. Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock Exposure | Status |
|---|---|---|---|---|---|---|---|
| P0 tool selection | blank map 首响应只能选择 control tool | Chat body/provider request construction | TaskSpace initial request | named-choice/visibility fixtures | tool-choice trace + final wire | provider probe only in J0 | complete; Docker live evidence passed |
| P1 ordered barrier | state change后的调用按最新状态执行 | native tool scheduler、TaskSpace preflight | multi-call provider response | order/failure/permission/attribution tests | barrier lifecycle events | none at exit | complete; positive/negative integration passed |
| P3 terminal transaction | finish 成功后直接发布 Agent final | taskspace_control handler、turn completion/history | last running node | success/reject/replay/provenance tests | terminal candidate events | none at exit | complete; two-request terminal fixture passed |
| explicit init mapping | key/id 方向无歧义且原子 next 恢复 | taskspace_control initialize output | initialize_map result | directional JSON output tests | bind=0、next_node_id=3、hard error=0 | none | complete; Docker live evidence passed |
| anti-collapse gate | 收益不来自节点减少 | benchmark topology + graph health | J4 Docker runs | topology fixtures | complex maps 6/6、8/11；无坍缩 | none | complete |
| repeated finish carrier | 同一响应可声明多个同名 finish并按最新状态执行 | provider + native sequence barriers | repeated-control response | provider probe、adjacent finish integration | provider返回2 calls；mock两节点完成 | provider probe only | complete |
| explicit finish target | 无 binding时显式 ready target原子 claim+finish | `finish_main_node_with_next` staged transaction | `taskspace_control.finish_node(node_id)` | success + pending no-side-effect tests | final live control failures=0、bind=0 | none | complete |
| cadence adoption | Agent主动使用 multi-finish或 finish+ordinary | Agent/provider output | Docker live sample | performance observer | multi-control=0、chained-finish=0、mixed=0 | none | hold; no benefit claim |
| schema-first continuation | 非终态 finish 在同一 tool 参数中必须携带 actions | `taskspace_control` schema + native ToolRouter | TaskSpace provider tool call | J6 schema/router/feedback tests | standalone/control-only/request delta | none at exit | planned in J6 |

## 11. Change-chain Logging Matrix

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation | Level | Consumer |
|---|---|---|---|---|---|---|---|
| hard-state tool selection | selected/released | `provider.hard_state_tool_selected` | provider rejected choice | `reason_code` | request_id/map_id | info/error | runtime/cache audit |
| barrier batch | queued/executing/completed | `tool.barrier_completed` | `tool.barrier_failed` | `failure_class` | request_id/barrier_id/call_id | info/warn | runtime/debug |
| dependent step | preflighted/executed/skipped | `tool.sequence_step_completed` | rejected/skipped | `skip_reason` | barrier_id/sequence_index/call_id/node_id | info/warn | Agent/debug |
| terminal candidate | staged/released/rejected | `taskspace.agent_final_released` | `taskspace.agent_final_rejected` | `gate_reason` | turn_id/node_id/call_id | info/warn | lifecycle audit |
| topology guard | captured/compared | `taskspace.topology_preserved` | `taskspace.topology_regressed` | `topology_delta` | run_id/pair_id/map_id | info/error | benchmark gate |

日志只记录结构、hash、状态和关联 id；final candidate 正文、用户输入和工具正文沿用现有受保护 history，不复制到诊断日志。

## 12. 风险与缓解

| Risk | Probability | Impact | Trigger | Mitigation | Fallback |
|---|---:|---:|---|---|---|
| 请求下降来自 Map 坍缩 | Medium | Critical | node/edge/phase coverage下降 | fixed-topology gate；复杂样本 graph health | 阻止收益声明，回退相关改动 |
| dependent call 按旧节点 preflight | Medium | High | attribution 指向 barrier 前节点 | preflight 下沉到 barrier 后，latest-state assertions | revert J2 |
| barrier failure 后仍执行 tail | Low | Critical | side effect after rejection | stop+skipped-output contract，权限/失败测试 | revert J2 |
| P0 改变 tools hash 破坏缓存 | Medium | High | request 2 tools/prefix mismatch | 首选 named tool choice；G1 wire/cache gate | revert J1 |
| terminal candidate 被误标 runtime 回答 | Medium | Critical | completion source/provenance错误 | exact Agent bytes + explicit source | revert J3 |
| runtime 开始推断动作顺序 | Medium | High | 自动插入/reorder/merge call | sequence order只来自 provider response | revert J2 |
| 旧 action-contract 借机回流 | Medium | High | native tools 被禁用或 JSON envelope出现 | forbidden scan + R5-F delete gate | 阻止 phase |

## 13. Open Questions

| Question | Owner | Resolution Gate | If Unavailable |
|---|---|---|---|
| DeepSeek 当前 ChatCompletions 是否完整接受 named `tool_choice` | provider integration | J0 | 只评估 hard-state visibility narrowing，并单独通过 tools-hash/cache gate |
| provider response item order 是否能稳定承载 barrier 前后 calls | provider/native scheduler | J0 | 暂停 J2，不恢复 action envelope |
| terminal carrier 使用 assistant content + tool call，还是显式 Agent candidate 字段 | turn completion | J0 | 暂停 J3，保留 J1/J2 |
| skipped tail 如何满足 provider history 的每个 call id output 配对 | native scheduler | J0 | 冻结明确 skipped output schema 后才能进入 J2 |

## 14. Decision Log

| Decision | Status | Reason |
|---|---|---|
| 实施 P0 hard-state tool selection | Accepted | 对齐现有硬状态和能力可见面，不增加任务语义 |
| 实施 P1 native ordered barrier | Accepted | 由 Agent声明顺序，runtime只机械执行和校验 |
| 实施 P3 Agent-authored terminal transaction | Accepted | 可消除最后固定往返，且 final 内容仍来自模型 |
| 使用 P2 Map 粗化降本 | Rejected | 有 Map 退化坍缩风险，破坏 TaskSpace 图化重组目标 |
| 恢复旧 action-contract sequence | Rejected | 禁用 native tools并引入额外 JSON/prompt 协议，不符合 R5 简洁边界 |
| runtime 根据工具成功自动 finish | Rejected | runtime 会取得任务推进语义所有权 |
| 把 mixed barrier=0 解释为 DeepSeek 不支持多工具 | Rejected | provider probe 和真实 ordinary batching 已反证；状态边界存在真实依赖 |
| 恢复精确的机械 action contract | Accepted for follow-up | 描述参数已实现的状态效果是工具自描述，不是 runtime 任务决策或语义提示 |
| 把 finish continuation 收进现有 tool schema | Accepted for J6 design | schema 才能约束本次 function 参数；顶层 sibling call 不能被该 schema 约束 |
| 新增独立 response frame/tool | Rejected | 扩散架构；J6 只演进现有 `taskspace_control` |
| 保留旧 finish 并在 response 后拒绝 standalone | Rejected | 属于后置惩罚，J5 已证明会制造重试和 no-op |

## 15. Plan Quality Checklist

- [x] P0/P1/P3 分 phase、可独立验证和回退。
- [x] P2 在目标、验收、日志和风险中均明确禁止。
- [x] 每个 phase 不依赖后续 phase 补齐自身退出证据。
- [x] correctness、request、cache、feedback、Map health 分账验收。
- [x] 生产路径、集成入口、测试和日志证据已进入完整性矩阵。
- [x] 权限、沙箱、失败停止、取消和 terminal provenance 已进入门禁。
- [x] 不保留兼容分支、旧 transport 或静默 fallback。
- [x] 正式收益只使用统一 Docker 环境和固定 Map 拓扑。

## 16. 当前暂停点

R5-F、R5-I0/I1/I2 和 J0/J1/J2/J3 已完成；J4 correctness/capability 已完成，但原收益门禁未通过。
机械 action contract 和显式初始化映射均已恢复并完成 fix validation，key/id 错误、冗余 bind 和原子
next binding 使用已收敛。当前剩余请求差来自 `N+1` control-only 往返和额外 ordinary actions；mixed
barrier 仍为能力存在但真实样本未采用。不得通过提示 Agent 少建节点、runtime 自动推进、拒绝合法调用
或节点粗化宣称请求收益。
