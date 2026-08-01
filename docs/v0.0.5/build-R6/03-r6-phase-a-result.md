# R6 Phase A 执行结果

- Created: 2026-07-15
- Status: Complete / Phase B Ready
- Scope: 合同冻结、现状审计、Docker 基线；无生产行为变更
- R5 Frozen Baseline: `d12818f`
- Machine Result: `benchmarks/taskspace/r6/phase-a-baseline-result.json`

## 1. 阶段结论

Phase A 的全部退出门禁已通过：rooted DAG 合同已成为机器可验证规格，当前实现的 31 个状态
ownership 项均有明确处置，Standard/R5 的 simple 与 branch/join 观察基线均在 Docker 硬边界中完成，
R6-A0 与 R5 的生产代码差异为零。

本阶段没有实现 R6 生产行为，也没有宣称 R6 性能收益。两组样本各运行一次，只用于冻结可重放的
诊断起点和验证现状假设。

## 2. 交付物

| 类别 | 交付物 | 结果 |
|---|---|---|
| 状态合同 | `rooted-dag-contract.json` | Root/Work/Finish、状态转换、图不变量、事务和错误码已冻结 |
| 合同 fixture | `rooted-dag-contract-fixtures.json` | 14 个正反例；合法图通过，非法 role/status/topology 被拒绝 |
| 合同测试 | `test-r6-rooted-dag-contract.ps1` | 独立校验合同、fixture、ownership 和基线定义 |
| Ownership | `phase-a-ownership-inventory.json` | 31 项、8 个 domain、0 unknown |
| 基线合同 | `phase-a-baseline-contract.json` | 固定 R5 binary、场景、证据和解释边界 |
| 基线结果 | `phase-a-baseline-result.json` | 两组 Docker pair 的结果、成本、缓存、Map 与证据 hash |

## 3. 合同冻结结果

R6 的唯一状态真相被固定为同一张 rooted DAG：

```text
Task Root (唯一 source，保持 OPEN)
  -> Work nodes (允许多入边、多出边)
  -> Finish (唯一 sink，由 Agent 显式 CLOSED 并提交总结)
```

Runtime 只验证结构、状态转换、revision 和事务原子性，不推断节点语义、不替 Agent 补边或选路。
初始化、变图和终结统一采用 candidate validate 后一次提交；拒绝时 revision、state hash 和图均不变。

现状审计确认 R5 已有可保留的机械基础，包括原始 Event Store、`MapEdge`、lease、checkpoint、只读
Viewer RPC 和 Docker harness。旧 `TaskStatus/MapStatus` 双重权威、可选依赖、外置 Finish、Root 推断、
语义化 NodeKind/status 和旧 projection 必须替换或删除，不能兼容保留。

## 4. Docker 基线结果

### 4.1 正确性与动作

| Sample | Arm | 结果 | Requests | Ordinary tools | Control | Failed tools | Map N/E/R |
|---|---|---|---:|---:|---:|---:|---:|
| simple | Standard | PASS | 5 | 7 | 0 | 0 | 0/0/0 |
| simple | R5 | PASS | 7 | 7 | 4 | 0 | 3/0/3 |
| branch/join | Standard | PASS | 9 | 16 | 0 | 1 | 0/0/0 |
| branch/join | R5 | PASS | 9 | 12 | 5 | 1 | 4/0/4 |

`N/E/R` 分别表示 node、edge、result。四臂均由 Agent 正常结束并通过外部验证；branch/join 两臂的
一次普通工具失败不属于 control/state/protocol failure，R5 的这些机制失败均为 0。

### 4.2 Token、缓存与时间

| Sample | Arm | Wall | Input | Cached | Uncached | Output | Total | Req2+ hit |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| simple | Standard | 18.883s | 34,731 | 33,152 | 1,579 | 1,955 | 36,686 | 94.94% |
| simple | R5 | 20.464s | 50,746 | 48,256 | 2,490 | 1,746 | 52,492 | 94.67% |
| branch/join | Standard | 44.443s | 92,952 | 86,144 | 6,808 | 4,844 | 97,796 | 92.38% |
| branch/join | R5 | 50.140s | 100,110 | 93,440 | 6,670 | 6,028 | 106,138 | 93.23% |

| Sample | R5/Standard requests | Wall | Input | Uncached | Output | Total | Cache delta |
|---|---:|---:|---:|---:|---:|---:|---:|
| simple | 1.40x | 1.08x | 1.46x | 1.58x | 0.89x | 1.43x | -0.27pp |
| branch/join | 1.00x | 1.13x | 1.08x | 0.98x | 1.24x | 1.09x | +0.85pp |

缓存没有零命中异常；simple 的 request 2+ 前缀保持为 Standard `4/4`、R5 `5/6`，branch/join 为
Standard `8/8`、R5 `7/8`。R5 各有一次 tool choice/cache shape transition，不能由一次运行推导为
结构性缓存回归。

Standard 旧链路没有输出 provider lifecycle timing，R5 可读到 simple `19.310s`、branch/join
`48.677s` 的模型请求累计时长。因此本轮只横向比较 wall time，不伪造 Standard 的 provider/tool
拆分；R6 独立 live arm 建立时应让三臂 timing 覆盖一致。

## 5. Map 结构证据

simple 的 R5 Map 为 `inspect_context`、`implement_fix`、`verify` 三节点、零边；branch/join 的 R5 Map
为 `read_project`、`diagnose_and_fix`、`verify_fix`、`summarize` 四节点、零边。两次 observer 唯一告警均为
`multi_node_map_without_edges`，没有 control、state、protocol、nested-action 或语义替换异常。

这支持 Phase A 的模型判断：R5 把依赖边当成可选信息，Map lifecycle 与状态机仍是平行结构。它不证明
Agent 缺乏规划能力，也不授权 Runtime 自动构图。R6 的修复方向仍是把合法拓扑作为 Map 工具的机械
合同，让拓扑内容完全由 Agent 声明。

## 6. R6-A0 第三臂

R6-A0 尚无生产代码，固定基线 `d12818f` 到 Phase A 完成态在 core、protocol、tools、app-server 和 tui
路径的 diff 数为 0。因此 R6-A0 是 R5 的 code-identity arm，复用 R5 数据；再调用一次 provider 只会
增加随机噪声，不能形成独立实现对照。首个独立 R6 live arm 保持在 Phase C 纵向切换后。

## 7. 对后续计划的校正

1. Phase B 继续保持生产不可达，只做纯领域模型、validator、transaction、event 和 reducer。
2. `TaskSpaceEventStore` 的自然上下文事件与 Map lifecycle event 必须分清所有权：前者忠实保存原文，
   后者只记录机械状态变更，不把两者合并为语义摘要。
3. Phase C 将 Event Store 的 Root owner 绑定到一等 `root_node_id`，但不改写 raw payload。
4. Phase C 必须重新生成 TypeScript protocol 类型；审计发现生成物仍残留旧语义字段，不能手工兼容。
5. Phase C/F 的 observer 要按 R6 合同输出 source/sink、可达性、cycle、深度和入/出度；零边多节点从
   当前告警升级为 R6 production invariant failure。
6. 当前成本只作为 R5 冻结基线，不调整 Phase B 架构，也不提前做 projection 或压缩优化。

## 8. 退出门禁

| Gate | 结果 |
|---|---|
| rooted DAG 机器合同和目标 schema 冻结 | PASS |
| 14 个正反例 fixture 与独立测试 | PASS |
| 31 项 ownership、8 domain、0 unknown | PASS |
| 旧路径均有 retain/adapt/replace/delete 决定 | PASS |
| Standard/R5 两样本 Docker 基线可读取 | PASS |
| R6-A0 与 R5 production code identity | PASS |
| 本阶段不改变生产行为 | PASS |

Phase A 完成并暂停。下一阶段是 Phase B 纯领域核心，不在本次执行范围内。
