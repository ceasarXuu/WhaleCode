# R5 Native Tool 状态屏障与终态收敛计划

## 1. 元数据

- Created: 2026-07-10
- Updated: 2026-07-11
- Version: v0.0.5 build-R5 follow-up
- Status: In progress - J0/J1/J2 complete, J3 next
- Owner / Responsible: WhaleCode core runtime
- Related Systems: provider tool choice、native tool scheduler、TaskSpace hard state、taskspace_control、turn completion、benchmark observer
- Related Links: `01-r5-phased-simplification-plan.md`、`13-r5-unified-docker-benchmark-and-logging-plan.md`、`coe/2026-07-10-22-56-r5-request-amplification.md`
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

**Fallback:** 任何 correctness、反馈、权限或 Map health 回退都阻止收益声明，并按 J3 -> J2 -> J1 的独立 commit 逆序回退。

## 9. Phase Gate Matrix

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Required Before Next | Decision |
|---|---|---|---|---|---|
| J0 | provider probe、fixed-topology fixture、failure contract | 不依赖 J1 code | capability/contract artifacts | 100% passed | proceed J1 |
| J1 | hard-state tool-choice fixtures、wire/cache trace | 不依赖 barrier | first-response and tools-hash evidence | 100% passed | proceed J2 |
| J2 | ordered barrier unit/integration/side-effect tests | 不依赖 terminal transaction | latest-state attribution and stop evidence | 100% passed | proceed J3 |
| J3 | terminal success/rejection/history tests | 不依赖 benefit repeats | completion provenance evidence | 100% | proceed J4 |
| J4 | Docker fixed-topology repeats、complex sample、adversarial review | 无后续 phase 补证 | performance/map/semantic report | 100% | proceed I3 |

## 10. Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock Exposure | Status |
|---|---|---|---|---|---|---|---|
| P0 tool selection | blank map 首响应只能选择 control tool | Chat body/provider request construction | TaskSpace initial request | named-choice/visibility fixtures | tool-choice trace + final wire | provider probe only in J0 | complete; Docker live evidence passed |
| P1 ordered barrier | state change后的调用按最新状态执行 | native tool scheduler、TaskSpace preflight | multi-call provider response | order/failure/permission/attribution tests | barrier lifecycle events | none at exit | complete; positive/negative integration passed |
| P3 terminal transaction | finish 成功后直接发布 Agent final | taskspace_control handler、turn completion/history | last running node | success/reject/replay/provenance tests | terminal candidate events | none at exit | planned |
| anti-collapse gate | 收益不来自节点减少 | benchmark fixed topology + graph health | J4 Docker runs | topology fixtures | map/node/edge/control metrics | none | planned |

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

R5-F、R5-I0/I1/I2 和 J0/J1/J2 已完成，当前进入 J3。不得通过
提示 Agent 少建节点、runtime 自动 finish 或节点粗化宣称请求收益。
