# R7 Phase A 当前状态与 Projection Ownership 审计

- Created: 2026-07-18
- Updated: 2026-07-18
- Status: Phase A Complete / Phase B Ready
- Scope: Projection ownership、context lifecycle、tool/observer/Viewer 接口；不改变生产行为
- R6 Frozen Baseline: `e29810158`
- R6 Runtime Commit: `2117de6e`
- Machine Contract: `benchmarks/taskspace/r7/projection-policy-contract.json`
- Machine Inventory: `benchmarks/taskspace/r7/phase-a-ownership-inventory.json`
- Baseline Contract: `benchmarks/taskspace/r7/phase-a-baseline-contract.json`
- Baseline Result: `benchmarks/taskspace/r7/phase-a-baseline-result.json`

## 1. 结论

R6 已经只有一份 canonical Rooted DAG 和一套共享工具/事件链，但 projection 进入 provider context
的策略仍与 epoch cache 实现绑定：

```text
canonical Map
  -> action_map/runtime builds R6 epoch snapshot text
  -> session state caches snapshot + anchor + prefix hash
  -> provider composer removes historical snapshot and reinserts cached baseline at anchor
  -> later canonical call/result/tool feedback append after the fixed baseline
```

这个结构不是 R7 的第四种模式，也不适合作为三种模式的公共抽象。R7-B 必须一次删除 epoch
cache/anchor/scope 所有权，保留同一 canonical Map、renderer 输入、Event Store、ordinary tool chain 和
observer，再把 context emission 变成唯一 `ProjectionPolicy` 决策点。

审计没有发现独立的 Viewer Map 存储或第二套 Rooted DAG。TUI/app-server 读取 canonical snapshot，
可以继续共用。`taskspace_control` 当前没有 `read_map` action；R7-D 需要把它加入所有策略共用的一套
schema，而不是只给 `map-request` 增加专属工具。

## 2. 当前生产数据流

```text
Agent taskspace_control / ordinary tool calls
  -> shared handler / ToolRouter / sandbox
  -> ActionMapRuntime validates Rooted DAG hard invariants
  -> canonical Map + TaskSpaceEventStore commit
  -> ActionMapRuntime::build_developer_context
       -> bootstrap snapshot, or
       -> render_active_projection(canonical map)
  -> Session::prepare_provider_visible_prompt_items
       -> remove TaskSpaceMapEpochSnapshotR6V1 items
       -> decide_taskspace_projection_epoch(scope, prefix hash, anchor)
       -> reuse or refresh one cached epoch projection
       -> compose projection at the epoch anchor
  -> provider request
  -> client/provider_wire scanner records section cost and active projection identity
```

R6 的 baseline 后仍按自然顺序追加 Agent message、control call/result 和 ordinary tool feedback，因此
它恢复了较高的 provider 前缀命中；代价是 baseline 固定后不再持续暴露最新全景，Agent 需要从后续
delta 自行重建当前状态。这是 R6 历史方案的明确行为，不进入 R7 enum。

## 3. Ownership 分布

| Domain | 当前 Owner | 当前事实 | R7 处置 |
|---|---|---|---|
| canonical Map | `action_map/rooted_dag` + `ActionMapRuntime` | Root/Finish/nodes/edges/revision 的唯一事实 | `retain_shared` |
| projection renderer | `action_map/projection.rs` | 同时包含 Map 渲染与 R6 epoch marker | B：保留纯渲染，替换 marker/role |
| projection input assembly | `ActionMapRuntime::build_*developer_context` | 从 runtime 状态构造 bootstrap/active 文本 | B：只暴露 canonical snapshot/renderer input |
| epoch cache | `state/taskspace_projection_epoch.rs` | snapshot、scope、anchor、prefix hash | B：删除 |
| session epoch state | `SessionState.taskspace_projection_epoch` | 每个 session 的缓存 projection | B：替换为 policy + 机械 cursor |
| provider context composer | `Session::prepare_provider_visible_prompt_items` | 过滤旧 projection、决定 epoch、插入 anchor | B：替换为共享 policy composer |
| Event Store | `TaskSpaceEventStore` | 保存自然 call/result、checkpoint 和 refs | 保留；删除 R6 marker 专属过滤 |
| tool contract | `taskspace_control_*` | 初始化、变图、转换、终结、展开、读 output ref | 保留共享；D 增加共享 `read_map` |
| provider observer | `client.rs`、`provider_wire_sections.rs` | wire section、count、identity、freshness | B 先支持 always，F 补齐三策略 |
| benchmark observer | `scripts/taskspace-benchmark/lib/*` | token/cache/projection/map 汇总 | B 先支持 always，F 收敛四臂报告 |
| compaction | Event Store checkpoint + replacement history | provider history 可替换，canonical Map 独立 | 保留共享；E 定义各 policy 新 epoch emission |
| resume/fork/replay | rollout reconstruction + Map replay | 重建 Map、自然历史、mode 和 lease | 保留共享；E 增加 policy 精确恢复 |
| Viewer transport | app-server/TUI snapshot reader | 只读 canonical Map snapshot | 保留共享，不接触 provider policy |

## 4. Renderer 与 Runtime

### 4.1 可保留能力

`action_map/projection.rs` 已具备可复用的机械能力：

- 从明确的 Rooted DAG 输入构造 nodes/edges/frontier/current；
- 输出 Map id、revision 和完整全局骨架；
- 对节点详情执行确定性折叠并记录 size breakdown；
- 为折叠详情生成 hash/ref；
- 对同一输入保持确定输出。

R7-B 不应建立第二个 renderer。需要替换的是 `TaskSpaceMapEpochSnapshotR6V1`、
`projection_role: epoch_baseline` 和 runtime 直接决定 provider developer context 的职责。

### 4.2 必须移出的职责

`ActionMapRuntime` 当前同时负责：

1. 判断 bootstrap/active scope；
2. 构造 provider developer text；
3. 写入 R6 epoch marker；
4. 记录 projection budget；
5. 在错误时构造 projection integrity context。

目标边界是 Runtime 只提供 canonical snapshot、hard-state error 和机械预算事实；renderer 只渲染；
policy/composer 决定 context emission。projection integrity 失败仍应忠实返回错误，但不能自行变成语义
建议或另一个 Map 事实源。

## 5. Epoch State 与 Provider Composer

当前 `TaskSpaceProviderProjectionEpoch` 保存：

```text
scope
context
anchor
prefix_sha256
```

`decide_taskspace_projection_epoch` 只处理 `Reuse/Refresh`，scope 只有 `bootstrap` 与
`active:<map_id>`。`Session::prepare_provider_visible_prompt_items` 会先删除历史 projection，再按 epoch
anchor 插入缓存 context。Map revision 变化本身不会刷新 epoch，这是 R6 固定 baseline 行为的核心。

R7-B 的替换结果必须是：

```text
canonical identity + trigger + session policy + policy cursor
  -> None | ReplaceLatest | AppendRevision | ReturnAsToolResult
```

旧 `TaskSpaceProviderProjectionEpoch`、`TaskSpaceProjectionEpochDecision`、prefix hash、anchor、scope 和
四类 `taskspace.projection_epoch_*` 日志全部在 B 删除，不保留 alias 或 fallback。

## 6. Context 生命周期

| 生命周期 | 当前行为 | R7 共享基础 | R7 策略差异点 |
|---|---|---|---|
| regular request | epoch projection 复用或刷新 | natural history 与 Map 独立保存 | emission policy |
| retry | 复用同一 prepared history/epoch | canonical revision 不变 | always replace；append 不重复；request none |
| compaction | Event Store checkpoint + replacement history，清 epoch | canonical Map/Event Store hash 不变 | 新 epoch 首个 emission |
| resume | rollout 重建 history、Map snapshot/checkpoint | 精确恢复 Map/mode/lease | 恢复 immutable policy 和 cursor |
| fork | 截取 history 并重绑定 owner/lease | canonical fork snapshot | 继承原 policy，不允许 Agent 切换 |
| rollback | 替换历史并清 epoch | Map/replay hard state | cursor 与 surviving revision 对齐 |

`map-request` 在 compaction/resume 只需要机械 Map handle，不得把它扩展成缩小 projection；
`map-append` 新 context epoch 可以从一份当前 revision snapshot 开始；`map-always` 每个 request 都读取
最新 canonical revision。以上只改变 context emission，不改变 Map 回放。

## 7. Tool Contract 审计

当前 `TaskSpaceControlArgs` 包含：

```text
initialize_map
mutate_graph
transition_node
finish_end
expand_nodes
read_output_ref
```

handler、typed parser、state mapping、sequence executor、单 patch slot、原始 ordinary feedback 和
Root/Finish hard gate 都是共享能力，三种 policy 必须使用同一 hash 的 tool schema。

当前缺口是没有直接读取完整 canonical Map 的 `read_map`。R7-D 增加该 action 时必须：

- 对三种 policy 同时可见；
- 调用同一 renderer；
- 返回 map id/revision/hash 和忠实 projection/ref；
- 不规定读取时机，不自动调用，不返回 next action；
- 不改变 empty Map、binding、lease、ordinary tool 或 terminal hard gate。

## 8. Observer 与 Benchmark

### 8.1 可保留观测

- final wire request 的 system/natural/projection/control/ordinary/tools/tool-choice/other 分区；
- provider usage 的 input/cached/uncached/output；
- request 级 shape、LCP、cache transition；
- projection bytes/token/hash/map id/revision；
- control call、state commit、ordinary tool 和失败计数；
- Map nodes/edges/frontier/open/Root/Finish/replay；
- Docker agent/validator/oracle 隔离和 binary attestation。

### 8.2 必须替换的观测假设

当前 scanner 把“TaskSpace active 时每 request 恰好一个 active projection”作为共用健康条件。R7 中
只有 `map-always` 满足这个条件：

| Policy | 合法 automatic projection 形态 |
|---|---|
| map-always | 每个 active request 一份最新 projection |
| map-append | history 中 revision snapshot 递增；无 revision 变化时不新增 |
| map-request | ordinary request 为零；显式 `read_map` 结果属于 tool feedback |

Phase F 前 observer 只能记录当前 R6 历史事实，不能提前用 always 断言判定另外两种 policy 失败。

## 9. Viewer 与 Protocol

`MapRuntimeMode` 当前只表达 Standard/Experiment，符合 R7 要求，不应扩展成三个 Runtime mode。R7
需要新增独立 `TaskSpaceProjectionPolicy` 配置/metadata enum，并在 session 创建时冻结；Viewer 只显示
解析后的 policy 与 canonical snapshot，不自行重建或缓存 provider projection。

app-server/TUI 的 TaskSpace read/status transport 没有 projection epoch 所有权，可以保留。生成的
TypeScript/protocol 类型在 policy metadata 落地后统一生成，不手工复制三个模式类型。

## 10. R6 Epoch Residue 处置

机器扫描发现的 R6 marker/epoch 相关路径分为四组：

| 组 | 代表路径 | 处置 |
|---|---|---|
| producer | `action_map/projection.rs`、`action_map/runtime.rs` | B：保留纯 renderer/input，替换 epoch 输出 |
| cache/composer | `state/taskspace_projection_epoch*.rs`、`state/session.rs`、`session/mod.rs` | B：删除 epoch，接入 policy/cursor/composer |
| history filters | `action_map/event_store.rs` | B：删除 R6 marker 专属过滤，保留 runtime-context ownership |
| core observer | `client.rs`、`provider_wire_sections*.rs`、runtime trace fields | 保留观测，B/F 改为新 marker 与 policy-aware 规则 |
| tests | projection/runtime/event-store/session/client/wire/reconstruction tests | B：替换成 renderer/policy/composer 决策矩阵 |
| benchmark | cost/metrics/pair/performance/provider identity/release scripts | B：接入 always 身份；F：补齐三策略与 release gate |

完整文件级 disposition 由机器 inventory 管理；合同测试会重新执行 marker 文件扫描，任何未列出的新
文件都会让 Phase A gate 失败。不存在 `unknown` 或“以后再看”的分类。

## 11. Phase A 基线

| Slot | Scenario | Arms | 目的 | 解释边界 |
|---|---|---|---|---|
| simple | `single-file-fast-fix` | Standard/R6 | 固定机制成本、request、cache 和闭合 Map | 单次只作诊断 |
| complex | `subscription-billing-repair` | Standard/R6 | 多文件自然工作路径、工具与 Map | 不代表 R7 三策略收益 |

两组均使用冻结 binary、同一 DeepSeek model、`reasoning_effort=max` 和 Docker hard boundary。Phase A
不会构造 R7 provider arm，因为本阶段没有生产行为变化；R7 的首个独立 live arm 是 Phase B 的
`map-always`。

## 12. Phase B 输入边界

Phase B 只允许以下纵向变化：

1. 新增共享 `TaskSpaceProjectionPolicy`、trigger/cursor/emission 纯合同；
2. 把当前 renderer 去除 epoch 语义，仍从同一 canonical Map 渲染；
3. 把 provider composer 切换到 `map-always`；
4. 删除 R6 epoch state、anchor、scope、marker filters 和日志；
5. observer 先支持 always freshness，不实现 append/request；
6. retry/resume/compaction 都读取最新 canonical revision。

Phase B 不修改 Rooted DAG、Event Store 语义、ordinary tools、权限、hook、Viewer snapshot 或 Agent
任务决策。B 完成时只有 `map-always` 可被生产配置选择；C/D 分别接入另外两种 policy。

## 13. Inventory Gate

| Gate | 最终状态 |
|---|---|
| renderer/runtime/session/state/protocol/tools 覆盖 | PASS |
| observer/benchmark/compaction/resume/fork/Viewer 覆盖 | PASS |
| R6 epoch marker 文件均有 disposition | PASS |
| classification 无 unknown | PASS |
| R6 epoch 明确不是第四 policy | PASS |
| 三策略机器合同 | PASS |
| Standard/R6 simple、complex Docker 基线 | PASS |
| 四个 arm 的 wire 离线重算 | PASS |
| 生产行为变化 | NONE |

Phase A 总 gate 已通过，最终证据记录在 `03-r7-phase-a-result.md`。Phase B 可以按 inventory 的
`replace_b`、`delete_b`、`adapt_b` 顺序开始共享 policy core 与 `map-always` 纵向切换。
