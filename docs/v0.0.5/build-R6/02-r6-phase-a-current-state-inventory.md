# R6 Phase A 当前状态与 Ownership 审计

- Created: 2026-07-15
- Updated: 2026-07-15
- Status: Complete
- Scope: Phase A inventory only / no production behavior change
- Machine Contract: `benchmarks/taskspace/r6/rooted-dag-contract.json`
- Machine Inventory: `benchmarks/taskspace/r6/phase-a-ownership-inventory.json`
- Frozen R5 Baseline: `d12818f`

## 1. 结论

当前 R5 不是一个错误实现的 rooted DAG，而是不同的模型：

```text
TaskState lifecycle
  + ActionMapInstance lifecycle
  + MapNode lifecycle
  + optional dependency edges
  + current binding
  + caller-selected terminal chain
```

状态机权威散落在 Task、Map、Node 和 terminal handler 四处。Map 的边已经表达
`from -> to` 依赖，并支持多入边，但依赖是可选能力；初始化和创建节点都允许空依赖，完成任务时
又可以把 Agent 传入列表的最后节点当作终点。因此 Map 还不是状态机本身。

R6 可直接继承的基础主要是忠实 Event Store、node-local refs、lease、tool sequence、snapshot hash、
read-only Viewer RPC 和 Docker benchmark。必须替换的是生命周期权威和 topology contract；不能靠在
projection 中推断 root/sink 或给旧 schema 增加兼容字段完成。

## 2. 当前生产数据流

```text
user/provider ResponseItem
  -> TaskSpaceEventStore records raw payload and owner
  -> Runtime creates mechanical blank TaskState + ActionMapInstance
  -> Agent calls initialize_then_actions
       -> nodes created one by one
       -> dependency_node_ids converted to MapEdge
       -> selected current node bound
  -> ordinary tools record NodeEvent/NodeResult refs
  -> Agent calls finish_nodes/create_node repeatedly
  -> Agent calls finish_then_end(finish_node_ids[], final_candidate)
       -> supplied chain completed
       -> MapStatus=Completed
       -> TaskStatus=Completed
       -> active task/map cleared
  -> snapshot/projection/viewer serialize the resulting parallel structures
```

原子 clone-then-commit 已存在于部分 initialize/terminal 路径，这是可复用的事务习惯；但它校验的是
旧模型，不会保证唯一 Root、唯一 Finish、全节点双向可达或唯一 source/sink。

## 3. 权威分布

| Authority | 当前 Owner | 当前事实 | R6 处置 |
|---|---|---|---|
| 用户输入和自然工具历史 | `TaskSpaceEventStore` | 保存 raw payload、role、call ID、success 和 owner | 保留，扩展 Map lifecycle events |
| Task 生命周期 | `TaskState.status/active_map_id` | 可独立 Active/Pending/Completed | 删除可变权威，进入 Root/Finish |
| Map 生命周期 | `ActionMapInstance.status` | 可独立 Active/Completed/Abandoned | 删除可变权威，由图内状态派生 |
| 节点执行状态 | `MapNode.status` | 所有节点共用一套状态 | 替换为 role-specific 状态合同 |
| 依赖关系 | `MapEdge` | 已是 `from -> to`，但可以完全不存在 | 保留边结构，提升为强不变量 |
| 当前工作 | `current_main_node_id/lease_id` | 图外 binding | 保留机械 binding，限制为 work frontier |
| 终点 | `finish_then_end` 调用参数 | 调用列表末项临时承担 terminal | 替换为 Map 固有 Finish |
| 完成总结 | `final_candidate` | Agent 提供，已有原样传递基础 | 保留 Agent 所有权，归档到 terminal event |
| Provider 可见 Map | `projection.rs` | task/map status + root refs + nodes/edges | 替换为同一 rooted DAG 纯构造 |
| Viewer 状态 | snapshot RPC | 只读，无第二存储 | 保留 transport，更新 snapshot/render |

## 4. 模型差异证据

### 4.1 Root 不在图中

`TaskState.source_event_ids` 和 projection 的 `root_source_event_ids` 表示任务来源，但它们不是
`MapNode`。压缩层只能把零入度节点当作 graph roots；无边 Map 中每个节点都会被视为 root。

R6 将 Task Root 变成一等节点，`root_node_id` 只允许指向它。用户目标和 source refs 归属于 Root，
不再从 TaskState 与 projection header 平行暴露。

### 4.2 Finish 不在图中

当前 `finish_then_end` 接收 `finish_node_ids[]`，Runtime 顺序完成这些节点，并在最后关闭外置
Task/Map。Map 本身没有稳定 `finish_node_id`，也不能证明每个节点能到达终点。

R6 初始化时就创建唯一 Finish。Runtime 可以机械派生其 READY，但只有 Agent 显式 terminal 事务
可以关闭 Finish 与 Root。

### 4.3 依赖边是可选项

初始化参数和 `create_node` 的 `dependency_node_ids` 都使用默认空数组。Runtime 已检查 missing ref、
self-dependency、duplicate dependency 和 cycle，也支持多个 dependency；但无依赖节点会直接 READY，
并且没有唯一 source/sink、Root 全可达、全节点可达 Finish 的校验。

R5 S4.2 的 24 次正式矩阵和 active-prefix 观察都没有自然依赖边；复杂样本仍形成 4 至 5 个节点、
0 条边。这不是 S4.2 折叠器的问题，而是现有 Map contract 没有把 topology 作为状态机底线。

### 4.4 动态创建无法始终保持单出口

`create_node` 只追加节点和从依赖节点指向新节点的边。假设现有图已经有唯一 Finish，在 Finish 前
插入新工作通常需要同时 remove/add edges。单节点 API 会产生暂时或永久的额外 sink。

R6 用 Agent 声明的 `mutate_graph(add_nodes/add_edges/remove_edges)` 在 candidate graph 上全量验证，
Runtime 不猜测如何改接。

## 5. Ownership 处置摘要

机器 inventory 共 31 项，覆盖 8 个 domain，没有 `unknown`。

| Classification | 主要对象 | 目标 Phase |
|---|---|---|
| `retain_mechanical` | `MapEdge`、Event Store、checkpoint、只读 RPC、资源/诊断日志 | B-H |
| `adapt` | Map 容器、lease、binding、events/results、codec、snapshot delta、viewer、observer | B-H |
| `replace` | TaskState、NodeKind/Status、active task/map、control schema、snapshot、projection | B-F |
| `delete` | TaskStatus、MapStatus、tasks HashMap completion authority | C |
| `regenerate` | app-server protocol TypeScript snapshot types | C |
| `hold_rebase` | R5 S4.2 detail fold | G |

完整逐项 owner、路径、原因和目标阶段见机器 inventory。Phase C 的生产切换门禁必须扫描这些
`delete/replace/regenerate` 项，避免旧权威残留。

## 6. 可保留的 R5 工程基础

### 6.1 Event Store

`event_store.rs` 已经完成以下正确基础：

- 原始 `ResponseItem` 作为 `raw_payload` 保存；
- Global/Root/Node owner 明确；
- tool call/output 的 call owner 可配对；
- checkpoint 保存覆盖范围、hash、output refs 和 replacement items；
- restore 检查 sequence、metadata 和 checkpoint hash；
- linearize 不依赖 Runtime 语义重写。

R6-B/E 应扩展 Map lifecycle event/reducer，不重写自然上下文保存机制。

### 6.2 原子事务习惯

`initialize_map_for_main` 和 terminal chain 已采用 clone candidate、成功后整体替换的模式。R6-B 可以
提取为纯领域 mutation，而不是从零发明事务协议。

### 6.3 反馈与执行载体

`taskspace_control` handler、ordered tool sequence、stop-after-first-failure、NodeEvent/raw ref 和
`TaskSpaceControlResultV2` 的 committed state 回执可以保留机械能力。R6 只替换 Map action schema 和
状态错误码，不恢复语义纠错提示。

### 6.4 Snapshot/Viewer/Docker substrate

snapshot delta 的 hash/reconstruction、`thread/taskspace/read` 只读 RPC、localhost Viewer transport 和
Docker agent/validator/oracle 隔离都可继续使用。新 snapshot 不兼容旧 snapshot，TypeScript types 由
Rust protocol 重新生成。

## 7. 必须删除的旧假设

Phase C 完成后以下生产形态必须不可表达：

```text
TaskStatus / MapStatus 独立完成写入
zero-edge multi-node Map
通过零入度扫描推断 Root
通过 finish_node_ids 最后一项推断终点
create_node 在不重接旧边时追加悬挂 sink
snapshot restore 猜测 root/finish
NodeKind -> ordinary tool action permission
```

不为旧 session、snapshot 或 benchmark artifact 增加读取兼容。历史报告保留，运行数据按 R6 schema
重新生成。

## 8. Phase B 输入边界

Phase B 只实现生产不可达的纯领域核心：

1. `TaskSpaceMap`、`MapNode(role)`、`MapEdge`、revision；
2. 图不变量 validator；
3. role/status transition 与 Root-open readiness；
4. initialize/mutate/terminal candidate transaction；
5. canonical Map lifecycle events 和 reducer；
6. property/fixture tests。

Phase B 不改 provider tool schema、不读旧 snapshot、不接 active projection、不运行 R6 live behavior。
只有纯核心全部通过后，Phase C 才做一次纵向生产切换。

## 9. Phase A 基线选择

| Slot | Scenario | 选择原因 | 不宣称内容 |
|---|---|---|---|
| simple | `single-file-fast-fix` | 读取、单文件修复、测试，适合观察固定机制成本 | 不要求形成深 Map |
| branch-join observation | `multi-file-order-pipeline` | 规则、测试和多个模块互相关联，存在自然并行调查与汇合机会 | prompt 不要求 fork/join，零边不算 validator 失败 |

Phase A 使用固定 R5 binary 同时跑 Standard/R5。R6-A0 尚无生产代码变化，`d12818f..R6-A`
对 core/protocol 的 diff 必须为零，因此 R6-A0 是 R5 的 code-identity arm，不重复调用 provider 制造一个
仅标签不同的随机样本。Phase B 仍无 production 差异；首个独立 R6 live arm 从 Phase C 纵向切换后开始。

## 10. Phase A 退出判断

| Gate | 状态 |
|---|---|
| rooted DAG 机器合同冻结 | PASS |
| 正反例 fixture 与独立合同测试 | PASS |
| ownership domain 覆盖完整，无 unknown | PASS |
| 旧路径全部有 replace/delete/retain 决定 | PASS |
| Standard/R5 两样本 Docker 基线 | PASS |
| R6-A0 与 R5 production code identity | PASS |

完整运行结果、指标和证据 hash 见 `03-r6-phase-a-result.md` 与
`benchmarks/taskspace/r6/phase-a-baseline-result.json`。
