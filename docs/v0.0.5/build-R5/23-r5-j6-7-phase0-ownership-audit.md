# R5-J6.7.0 任务上下文所有权审计

- Date: 2026-07-12
- Phase: R5-J6.7.0
- Status: Complete
- Scope: TaskSpace 任务项写入、provider 读取、压缩、恢复、退出和重复载体基线

## 1. 结论

当前 TaskSpace 没有任务上下文唯一事实源。`ConversationHistory` 保存原始 provider 协议项，
`ActionMap` 另存普通工具结果 preview/result/runtime feedback，固定 epoch projection 又把其中部分正文
复制为 developer message。nested action 的 call/output 还被包在 outer control arguments/output 中。

J6.7.2 的原子切换范围已经完整识别，不存在 owner 为 `unknown` 的 production 任务项入口。目标边界冻结为：

```text
global system/developer/environment/tool schemas -> ConversationHistory / turn context
TaskSpace task user/assistant/tool/control items   -> TaskSpace Event Store
provider TaskSpace input                          -> deterministic event linearizer
rollout                                            -> canonical event/snapshot persistence
```

不引入 semantic similarity、Runtime summary、兼容双写或 silent fallback。

## 2. 当前所有权矩阵

| 内容 | 当前写入入口 | 当前 provider 来源 | 当前副本 | 目标 owner | 主要风险 |
|---|---|---|---:|---|---|
| global system/developer | initial/context update | `ContextManager::for_prompt` | 1 | global history | 误迁入任务 Store |
| 原始 user task | user/pending/raw injection | base history | 1，另有 goal 复述 | root event | role/附件/owner丢失 |
| assistant message/reasoning item | stream completed item | base history | 1 | root/current node event | phase/end_turn 丢失 |
| direct tool call | stream item recorder | base history | 1 | current node event | arguments/call_id 丢失 |
| direct tool output | `record_response_input_item` | base history | 1 + NodeEvent + projection | current node event | feedback重复或截断 |
| control call | stream item recorder | base history | 1，状态另写 Map | current node/root event | 状态无 outer call_id |
| control output | aggregate output recorder | base history | 1，nested 正文聚合 | control result event | nested pair 被扁平化 |
| nested call | outer control arguments | outer call | 1 + 临时派生 item | current node event | 无独立原生 history pair |
| nested output | outer aggregate output | outer output | 1 + NodeEvent/projection | current node event | 同一正文多载体 |
| gate failure | failure output | base history | 1，部分再写 runtime feedback | typed failure event | reason 双表达/call_id 缺失 |
| result summary | control args + NodeResult.body | control + projection | 2 | 可选 Agent conclusion event | Runtime/Agent事实混同 |
| large output | history 截断/ref + NodeEvent ref | history/ref | 多 metadata 副本 | tool event + artifact ref | ref不可恢复 |
| active projection | turn context update | developer message | 1，每 epoch 固定 | Map index/checkpoint | 复制可见 raw 正文 |
| compaction summary | compact replacement history | base history | 1，Map独立保留 | checkpoint event | Store/history覆盖范围分裂 |
| ActionMap snapshot | Map runtime event | 不直接作为原生 item | 1 | Map结构快照 | 与event sequence不一致 |

## 3. Production 调用链

### 3.1 写入与 provider

1. user：`session/turn.rs:381` -> `Session::record_user_prompt_and_emit_turn_item`
   -> `Session::record_conversation_items` (`session/mod.rs:3270`)
   -> `ContextManager::record_items` (`context_manager/history.rs:99`)。
2. assistant/tool call：`stream_events_utils.rs:211-237` 在执行工具前记录 provider 原始 item。
3. tool output：`session/turn.rs:3566-3574` 执行 sequence 后通过
   `record_response_input_item` (`session/turn.rs:3083`) 写 history。
4. provider：`session/turn.rs:491-495` clone history -> `for_prompt` ->
   `prepare_provider_visible_prompt_items`；retry 路径在 `session/turn.rs:1250-1258` 重走同一来源。
5. composer：`session/turn.rs:1560-1637` 依赖 projection marker 分类并过滤 legacy/shadow/large raw，
   不是从 Map/Event Store 构造任务上下文。
6. pending input、hook injection 和 public `CodexThread::inject_response_items` 也能写入 message/tool item；
   J6.7.2 必须在最终汇聚入口分类，不能只改普通 user turn。

### 3.2 direct 与 nested tools

1. direct ordinary tool 在 `tools/parallel.rs:163-228` 执行前后调用 ActionMap attribution；
   history 仍由通用 stream/output 路径保存完整 call/output。
2. ActionMap 只持久化 result preview：`action_map/runtime.rs:1685-1702` 写 `NodeEvent.body`、
   `visible_excerpt`、`raw_ref` 和 `call_id`，不保存原始 call arguments。
3. nested call 由 `taskspace_control_args.rs:113` 机械合成，ID 为
   `${outer_call_id}:nested:${index}` (`tools/sequence.rs:222`)。
4. nested item 不独立写 history；`tools/sequence.rs:311` 把结果再次包装进 outer control output。

### 3.3 control 与 gate

1. `taskspace_control` outer call/output 使用 base history 的 call_id 配对。
2. handler 状态 API 未携带 outer call_id；Map 只保存状态效果和独立 result/trace ID。
3. parser/handler failure 由 `TaskSpaceGateRecoveryV1` 与文本共同表达；普通工具 hard gate 还可能写
   `runtime_feedback`，但该 NodeEvent 的 `call_id` 为 `None`。
4. 当前可靠协议配对只存在于 base history 内，无法跨 history/Map/projection join。

### 3.4 projection

`record_context_updates_and_set_reference_context_item` (`session/mod.rs:3812`) 调用
`ActionMapRuntime::build_developer_context`，再由 `action_map/projection.rs:20` 生成 developer message。
projection 从 NodeEvent `visible_excerpt` 复制正文，因此它是第二个语义载体而非纯 Map 索引。

## 4. 生命周期路径

| 路径 | 当前行为 | J6.7 切换要求 |
|---|---|---|
| activate | `SetMapRuntimeMode(Experiment)` 创建机械空 Map，既有 history 不 move | 将当前任务段原子 move 到 root events |
| normal turn | 所有 item 写 base history；Map旁路记 preview/state | task item只写 Event Store |
| retry | 重新 clone base history | 从同一 canonical linearizer 重建 |
| compaction | clone/summary 后 `history.replace`；Map snapshot不变 | event range checkpoint，禁止双覆盖 |
| resume | rollout分别找最新 mode/snapshot；旧 snapshot 可覆盖更新 mode | 按统一 sequence恢复，mode不能被旧snapshot覆盖 |
| fork | 原样恢复旧 owner/lease/output-ref，未重绑新 thread | 重绑owner或显式拒绝，ref必须可恢复 |
| rollback | `drop_last_n_user_turns` 只改 history，仍恢复被回滚Map snapshot | 事务性裁剪 event range 与 Map引用 |
| exit | 底层可设Standard，但不清projection；provider仍可能暴露TaskSpace tools | 从events重建Standard history并清活跃载体 |
| reborn | 只请求新 task path，不清除旧 history | 新 task/map owner边界明确，旧events可追溯 |
| subagent | child tool结果可归属 parent Map，child history独立 | parent_call_id/owner明确，不复制正文 |
| maintenance barrier | snapshot含barrier，restore当前无条件清空 | snapshot/event合同无损恢复硬状态 |
| output ref | 绑定当前rollout sidecar目录，fork只复制rollout item | ref storage身份与thread fork解耦 |

## 5. ResponseItem 类型清单

J6.7.1 codec 至少覆盖以下 provider/task 类型，unsupported 类型必须显式失败：

- `Message`：user/assistant/developer，text/image，`id/end_turn/phase`。
- `Reasoning` 与 provider 可见 reasoning metadata。
- `FunctionCall` / `FunctionCallOutput`。
- `CustomToolCall` / `CustomToolCallOutput`。
- local shell call/output。
- MCP call/output。
- tool search call/output。
- image generation、web search 等原生 ResponseItem。
- output ref、truncation metadata 和 success 状态。
- control transition/gate failure 的 TaskSpace typed event。

`GhostSnapshot`、纯 UI event、telemetry 和 Map observability 不作为 task semantic event；rollout 必须继续保存
恢复所需的 snapshot/checkpoint metadata。

## 6. 精确重复观测

新增 `performance-duplication.ps1`，只按完整 payload SHA-256、原始 output body SHA-256、call_id
和 provider message content SHA-256 统计，不判断自然语言相似度。

| Sample / Mode | Result | Requests | Input | Final wire content dup | Final dup bytes | Rollout output body dup |
|---|---|---:|---:|---:|---:|---:|
| count-call-stack / Standard | solved | 8 | 57,857 | 3 | 2,395 | 0 |
| count-call-stack / R5 | solved | 11 | 90,412 | 6 | 4,086 | 1 / 283 bytes |
| multi-file-order-pipeline / Standard | solved | 10 | 96,520 | 6 | 3,235 | 0 |
| multi-file-order-pipeline / R5 | solved | 17 | 225,040 | 10 | 3,692 | 0 |

wire content duplicate是结构上界，可能包括业务上确实相同的独立输出；它不直接等价于错误语义重复。
J6.7 只把该指标用于切换前后机械对照，不据此让 Runtime 删除或摘要正文。

## 7. Standard / R4 / R5 基线边界

- 当前 Standard/R5 使用 2026-07-12 同日 Docker artifact：
  `target/r5-k1-input/run/count-call-stack/.../20260712-084344-432` 与
  `target/j6-complex-a/order-pipeline/.../20260712-041646-435`。
- R4 可执行快照已不在当前工作区，不能诚实同机重放。`count-call-stack` 仅保留历史 solved 基线：
  wall 154,525ms、11 tools；request/token/cache 不可用。
- `multi-file-order-pipeline` 无同口径 R4 artifact，记为 unavailable，不补造数据。
- Phase 0 的目标是冻结当前 ownership 与 payload 基线，不用历史 R4 缺失阻断 canonical contract；
  后续收益结论不得声称已完成该复杂样本的三向成本对比。

## 8. J6.7.1 输入门

- production task item入口 owner：100% 已识别，`unknown=0`。
- activation/exit/resume/fork/rollback/compaction/subagent：100% 已识别。
- provider 原生 pairing风险：已识别，J6.7.1 必须逐字段往返。
- observer exact-hash contract：self-test通过，现有两个 Docker run可读取。
- 禁止项：semantic similarity、production shadow dual-write、summary替代raw、silent fallback。

## 9. 已定位的切换阻断缺陷

| ID | Severity | 表现 | 根因 | Owner phase |
|---|---|---|---|---|
| J6.7-B01 | P0 | 退出后provider仍走TaskSpace工具分支 | active projection未清；分支看marker/budget而非mode | J6.7.2 |
| J6.7-B02 | P0 | resume把已退出会话恢复为Experiment | 最新snapshot覆盖更新后的ModeChanged | J6.7.2 |
| J6.7-B03 | P0 | rollback后被撤销Map状态仍恢复 | snapshot选择不受rollback segment约束 | J6.7.2 |
| J6.7-B04 | P0 | fork后lease owner与新thread不匹配 | snapshot owner/lease原样恢复 | J6.7.2 |
| J6.7-B05 | P0 | maintenance barrier恢复后消失 | `restore_snapshot`无条件clear | J6.7.2 |
| J6.7-B06 | P1 | compaction丢原始role/order/tool pair | compactor直接总结raw history | J6.7.4 |
| J6.7-B07 | P1 | 无ref的大输出连同call被provider删除 | composer按output call_id成对omit | J6.7.4 |
| J6.7-B08 | P1 | subagent fork形成父/子两个runtime副本 | child control与ordinary tool归属路径不一致 | J6.7.2 |
| J6.7-B09 | P1 | fork后父output-ref不可读 | sidecar路径绑定当前rollout文件名 | J6.7.4 |

上述缺陷均已有精确production路径，不再是 discovery unknown。J6.7.1 只证明无损承载能力；
J6.7.2 未同时关闭 B01-B05/B08 时不得宣称 canonical cutover 完成，J6.7.4 未关闭 B06/B07/B09
时不得宣称 compaction/ref 完成。

J6.7.0 达到退出条件，允许进入 J6.7.1。
