# R6 根式 DAG 状态机分阶段实施计划

> 本计划从 `00-r6-rooted-dag-state-machine-charter.md` 派生。R6 不兼容旧 TaskSpace 数据，
> 不在生产链路长期并存两套模型。实施顺序先冻结合同和基线，再构建可测试的新核心，随后完成
> 一次纵向切换，最后才重建 projection、性能和压缩结论。

## 1.1 元数据

```text
Created: 2026-07-15
Updated: 2026-07-15
Version: v0.0.5 build-R6
Status: Phase A Complete / Phase B Ready
Owner / Responsible: WhaleCode core runtime / TaskSpace
Risk Level: Critical
Plan Type: Full
Execution Order: A -> B -> C -> D -> E -> F -> G -> H
R5 Frozen Baseline: d12818f (S4.2 HOLD)
Compatibility Policy: none
```

## 1.2 执行摘要

R6 不是继续优化 R5 Map 的展示效果，而是先纠正状态模型。完整路径为：

```text
A. 冻结合同、盘点差异、建立三臂基线
B. 构建纯领域核心：Root/Work/Finish + validator + reducer
C. 一次纵向切换生产链路，并删除旧 Task/Map 双重状态
D. 完善 Agent 声明的原子图变更、并发前沿与多前置依赖
E. 完成显式终结、事件溯源、恢复/分叉和故障原子性
F. 收敛 projection、tool schema、反馈与上下文唯一所有权
G. 重建成本基线，再逐项评估压缩；不继承 R5 的无效收益结论
H. 全量回归、经授权的对抗性审查和 R6 收口
```

Phase B 允许新核心作为测试可见、生产不可达的短期 staging 模块；这不是兼容层。Phase C 必须
在同一阶段内完成生产切换并删除旧生产路径，不允许用 feature flag 长期双跑。

## 1.3 当前实现与目标差异

| 区域 | 当前 R5 实现 | R6 目标 | 主要位置 |
|---|---|---|---|
| 任务状态 | `TaskState.status` 独立维护 | Root/Finish 唯一决定任务生命周期 | `core/src/action_map/map.rs` |
| Map 状态 | `MapStatus` 独立维护 | 由图内状态派生，不可独立写 | `core/src/action_map/map.rs` |
| Root | 无显式节点；projection 另带 source refs，压缩层扫描零入度节点 | `root_node_id` 指向唯一 `task_root` | `map.rs`, `projection.rs` |
| Finish | `finish_then_end` 的有序列表最后项临时承担终点 | `finish_node_id` 指向唯一 `finish` | `taskspace_control_*` |
| 边 | `dependency_node_ids` 默认空；零边节点合法 | 除 Root 外必须有入边，除 Finish 外必须有出边 | `runtime.rs`, control args/schema |
| 完整性 | 校验引用、自环、重复、cycle 等局部规则 | 再增加唯一 source/sink、全可达和角色状态一致性 | `runtime.rs` |
| 图变更 | `create_node` 可单独追加节点和依赖 | Agent 声明 add/remove 的原子图事务 | `taskspace_control` |
| Ready | 无依赖节点自动 ready | Root-open 特例 + 全部普通前驱 completed | `runtime.rs` |
| Snapshot | task/map/nodes/edges 并列，Root/Finish 无一等 ID | 单一 Map snapshot + root/finish IDs + event revision | protocol/action_map snapshot |
| Projection | 已趋于薄视图，但仍适配多 root/零边 Map | 同一 canonical DAG 的纯构造 | `projection.rs` |

表中 `core/` 均指 `third_party/codex-cli/codex-rs/core/`。R6 会尊重 upstream vendor 边界，但
Whale 自有新增模块仍按单文件不超过 500 行拆分。

## 1.4 冻结的工程决策

1. 结构角色只有 `task_root | work | finish`；现有 inspect/implement/test 等语义分类不能再决定工具权限。
2. 边只有依赖语义，不再增加独立 parent/hierarchy 关系。
3. 多入边、多出边合法；环、孤立节点、额外 source/sink 非法。
4. Root 初始化后保持 OPEN；直接后继把 `root=open` 视作启动条件。
5. Finish READY 可以机械派生，CLOSED 必须由 Agent 显式提交最终总结。
6. 初始化和图变更都采用 candidate validation + atomic commit。
7. Runtime 不推断或修复拓扑，只反馈 invariant code 和零提交事实。
8. 旧 snapshot/session 明确不支持，不写 migration、adapter 或 silent fallback。
9. Event Store 是唯一历史，snapshot/delta/checkpoint 是可校验加速结构。
10. Projection 始终保留全图骨架；压缩只能处理局部详情，骨架超限另行专项。

## 1.5 全阶段验收规则

每个 Phase 必须满足以下共同门禁：

- 变更前记录根因假设和预期收益，变更后用测试/日志验证；
- 新增功能或 bug 修复同步增加结构化日志，日志不包含 Runtime 主观语义；
- 执行相关 unit、integration、snapshot/replay 和 Docker smoke；
- 每个小主题独立 commit 并 push，不创建新分支；
- 阶段结束时 git clean；
- 不保留失败方案、兼容分支、长期 feature flag 或静默 fallback；
- 若阶段门禁失败，回退该阶段 production 接线，不用补丁掩盖模型错误；
- 代码变化完成后先汇报，并询问用户是否授权对抗性审查。

## 1.6 横向样本与指标合同

### 1.6.1 固定对照臂

| Arm | 定义 |
|---|---|
| Standard | 同模型、同 provider、同 Docker substrate 的 Standard 模式 |
| R5 | 固定 commit `d12818f` 的 TaskSpace，不随 R6 改动漂移 |
| R6 | 当前 Phase 的候选 commit |
| R6 Previous | 从 Phase C 起额外保留上一已接受 R6 Phase，用于归因 |

三臂通过 commit-pinned Docker image/worktree 执行，不切换当前开发分支，不使用宿主 pytest/Rust
环境。样本、环境、模型配置和 validator 固定；运行顺序轮换，避免缓存冷热顺序偏差。

### 1.6.2 每阶段最低样本

每个 Phase 选择 1 至 2 个适合本阶段的客观样本，每个 Arm 先执行 1 次快速门禁：

- `simple`: 小范围读取、单次 patch、单次验证；防止机制成本和简单任务回归；
- `branch-join`: 两条可并行调查/实现路径汇合到验证；验证 fork/join 和多前置依赖；
- `rework`: 首次验证失败后 Agent 修改图并重做；验证原子图变更和反馈；
- `resume`: 中断、snapshot、恢复、继续并显式终结；验证 replay；
- `long-map`: 自然长任务；只在 G 阶段观察全局骨架和详情压缩。

样本不得提示 Agent 应创建多少节点、如何连接边、何时展开或如何通过 validator。确定性 fixture
用于证明机制，live sample 用于观察自然采用，两者结论不得混用。

### 1.6.3 固定指标

结果表必须包含单次值；正式门禁必须包含总和、均值、中位数：

```text
correctness / external validation / agent completion
provider requests / control calls / ordinary tool calls / failed tool calls
input / cached input / uncached input / output / total tokens
weighted cache hit rate / request-level hit shape
wall time / provider time / tool time
map revisions / nodes / edges / max depth / max indegree / max outdegree
root count / sink count / unreachable count / cycle count
graph mutation accepted/rejected/partial commit
projection bytes/tokens / duplicate sections / raw ref recovery
```

## 1.7 Phase A：合同冻结、差异审计与基线

**目标**：在改代码前，把状态、图、工具、事件和错误合同冻结为机器可验证规格。

实施项：

1. 建立 `r6-rooted-dag-contract.json`：节点角色、状态转换、图不变量和错误码。
2. 建立当前生产路径 ownership 图：model、runtime、handler、protocol、projection、snapshot、viewer。
3. 列出所有旧字段和路径，逐项标记 `replace/delete/retain-mechanical`，不允许 `unknown` 进入 C。
4. 冻结 `initialize_map`、`mutate_graph`、`bind/complete/block`、`finish_end` 的目标 schema 草案。
5. 固定 R5 commit 和 Standard/R5 simple + branch-join 各 1 次基线。
6. 记录外部依据、内部假设、风险和不采用方案。

退出门禁：

```text
合同覆盖全部合法/非法 role-status 组合。
错误码能区分 missing/duplicate root、missing/duplicate sink、cycle、unreachable、invalid transition。
旧结构 inventory 覆盖率 100%，没有 unknown owner。
基线 artifacts、rollout、request、cache、map digest 可复现读取。
本 Phase 不改变生产行为。
```

主要收益：避免边实现边重新定义“Map 是什么”，为后续无兼容切换提供可执行判据。

## 1.8 Phase B：纯领域核心与确定性验证器

**目标**：先把新模型做成无 provider、无 handler、无 projection 依赖的纯 Rust 领域核心。

建议模块边界：

```text
action_map/model.rs          Root/Work/Finish、Map、Edge、Revision
action_map/invariants.rs     DAG/source/sink/reachability/status validator
action_map/transitions.rs    readiness、node transition、terminal transaction
action_map/mutation.rs       candidate graph mutation and atomic commit
action_map/events.rs         canonical events and reducer
```

实施顺序：

1. 新增 Root/Work/Finish 数据模型和 role-specific 状态合法性检查。
2. 用成熟图算法实现 cycle 检查，并用正向/反向遍历验证双向全可达。
3. 实现纯函数 `validate(candidate) -> violations[]`，输出稳定错误码和 node/edge IDs。
4. 实现 reducer：仅由 canonical events 重建相同 Map/revision。
5. 实现初始化、图 mutation 和 terminal 的 clone-validate-commit 原子原语。
6. 用 property tests 生成 chain/fork/join/diamond/disconnected/cycle/multi-source/multi-sink 图。

生产约束：本阶段新核心只能被测试调用，旧生产路径不双写、不镜像、不比较后择优。Phase C 完成后
立即成为唯一核心；若 B 未通过，不进入 C。

退出门禁：

```text
合法 DAG fixtures 100% accepted，非法矩阵 100% rejected。
任意 reject 前后 state hash/revision 相同。
20-cycle event replay 与直接状态逐字段/hash 一致。
property tests 未发现 validator panic、漏环、漏孤立节点或非确定顺序。
Standard/R5/R6 simple 快速臂中 R6 production 行为仍与 R5 相同。
```

主要收益：把最危险的拓扑和事务逻辑从 provider/tool loop 中剥离，先证明机械正确性。

## 1.9 Phase C：生产链路纵向切换

**目标**：把 model、tool schema、runtime、event、snapshot 和 projection 一次切到新 Map，删除旧权威。

阶段内小提交顺序：

1. Protocol 增加 `root_node_id`、`finish_node_id`、role/status 和 revision 的新 snapshot/event 定义。
2. `taskspace_control` schema 改为显式初始化完整图；dependency 不再默认空。
3. 接入 Agent 声明的 `mutate_graph` 原子事务，替换可产生悬挂节点的旧 `create_node`。
4. Runtime 接入 Root-open readiness、work lifecycle 和显式 Finish 事务。
5. Event Store、snapshot/delta/checkpoint、projection 和 Viewer 同步切换到同一模型。
6. 删除 `TaskStatus`、独立 `MapStatus`、旧 terminal list 语义和 root 推断路径。
7. 删除旧 fixture/adapter，重新生成 R6 fixtures；旧 session 返回明确 unsupported schema fatal。

本阶段在 production 接线提交前允许新模块存在，但接线后不得有运行时模式开关。若整阶段未通过，
用 Git 回退到 Phase B/R5 生产路径，不在产品中保留半切换状态。

退出门禁：

```text
生产 Map 初始化后恰好一个 Root/Finish，全部节点均在 Root->Finish 路径上。
无 TaskStatus/MapStatus 可变完成权威；completion 只有一个派生来源。
旧 create_node、terminal-last-node、zero-edge production schema 不可表达。
旧 snapshot/session 明确失败，不被猜测迁移。
simple + branch-join 的 Standard/R5/R6 各1次均完成外部验证。
```

主要收益：消除双重状态和零边 Map，首次让 production Map 与状态机成为同一对象。

## 1.10 Phase D：动态图、并发前沿与 Agent 控制

**目标**：证明新模型不只适用于静态 chain，还能忠实承载 Agent 的 fork、join、rework 和并发工作。

实施项：

1. `mutate_graph` 支持批量 add nodes、add edges、remove edges；先全量校验再提交。
2. 明确运行中节点相关边的变更规则，禁止破坏已记录执行因果，但不评价变更语义。
3. Readiness 对多前驱采用全满足规则；一个前驱未完成时不得 ready。
4. current/lease/owner 必须指向合法 active frontier；Root 和 Finish 不承载 ordinary tool lease。
5. block/unblock/rework 由 Agent 发起，Runtime 只校验状态和引用。
6. 多 Agent 同 revision 变更使用 optimistic revision check，冲突返回机械 stale revision。
7. Viewer 展示真实多入边、多出边和 active frontier，不构造层级父子关系。

退出门禁：

```text
fork/join/diamond/rework fixtures 状态推进正确。
任何 mutation reject 都是 state_commit=false、partial_commit=0。
并发 stale revision 不覆盖较新图，不自动合并 Agent 意图。
branch-join + rework 三臂各1次；自然边数、深度和动作路径进入报告。
Agent 未建立理想图时只记录能力现象，不由 Runtime 自动补图。
```

主要收益：Map 能表达真实依赖，而不是退化为 Root 直连所有节点或散落节点列表。

## 1.11 Phase E：显式终结、Replay 与故障原子性

**目标**：确保唯一 Finish 是 Agent 手动闭合的终点，并可在中断、恢复和分叉后保持一致。

实施项：

1. `finish_end` schema 不接受任意 finish 列表，只作用于 map 固有 Finish。
2. final summary 必须由 Agent 提供并原样形成 terminal event；Runtime 不总结、不润色。
3. terminal 预检覆盖 Finish ready、无未完成必要节点、图不变量和 revision。
4. Finish close、Root close、terminal event、snapshot revision 在单一事务提交。
5. 在每个提交边界注入 crash/failure，验证恢复后不是部分闭合。
6. snapshot、delta、resume、fork、replay 和 corruption fatal 建立完整矩阵。
7. 删除任何“看起来完成”自动 end、自动选择最后节点或拒绝后策略提示。

退出门禁：

```text
没有 Agent terminal call 时 Finish 永不 CLOSED，Root 永不 CLOSED。
任一 terminal precondition 失败时 Root/Finish/revision/event 均不变化。
成功 terminal 后 Root/Finish 同 revision 闭合，summary 字节级保持 Agent 输入。
20-cycle replay/resume/fork 状态 hash 100% 一致；corruption 明确 fatal。
resume + rework 三臂各1次完成，无 reject loop。
```

主要收益：任务结束从外置约定变为图内、显式、可回放的唯一终点。

## 1.12 Phase F：Projection、工具反馈与上下文唯一性

**目标**：让 Agent 看到的是同一 canonical DAG 的忠实视图，不因新模型再次引入重复或语义注入。

实施项：

1. Projection 固定显示 Root、Finish、全 nodes/edges skeleton、frontier、current 和机械状态。
2. Root 详情保留用户目标/source refs；近端节点保留更多忠实 event/result refs。
3. 基础上下文中与 Map 重复的 task/state/control 内容删除，Map 成为首选 owner。
4. tool schema 只暴露一次；success/failure result 返回 committed revision、delta 和 violation codes。
5. ordinary tool feedback 保持原始 outcome/excerpt/ref，不由 projection 改写。
6. 删除策略性 next action、纠错文案、语义 coverage 和 action-class 权限映射。
7. 建立 provider wire section/hash/LCP 和 duplicate detector。

退出门禁：

```text
provider payload 中 Root/Finish/active state 各只有一个 authoritative section。
全节点/边骨架 coverage=100%，不存在额外推断 root/sink。
tool failure 的 exit/stderr/truncation/ref 与 Event Store 一致。
projection_strategy_hint_count=0，semantic rewrite count=0。
simple + branch-join 三臂各1次无 correctness 回退。
```

主要收益：新状态机不以额外上下文副本为代价，反馈层继续遵守语义透传原则。

## 1.13 Phase G：成本重基线与压缩策略重新进入

**目标**：在图模型稳定后重新测量 request、token、cache、时间和 Map 增长，再决定压缩是否值得。

实施顺序：

1. 先冻结 R6-B0：无新增压缩策略的 Rooted DAG projection 基线。
2. simple、branch-join、long-map 执行 Standard/R5/R6-B0，各 Arm 3 次并轮换顺序。
3. 分解每 request 的 system/tool schema/natural history/map projection/tool feedback/token 占比。
4. 检查缓存前缀 LCP、首请求冷启动、同 shape 后续命中和日志观测口径。
5. 重新评估 R5 S4.2；只能作为一个独立候选策略，不能直接视为 R6 默认行为。
6. 每次只启用一个详情策略，对比 Standard/R6-B0/R6-Previous/R6-Candidate；测试后暂停汇报。
7. Root/Finish/全图骨架永不分页；骨架本身超限保持显式错误并另建后续专项。

退出门禁：

```text
三样本正式矩阵结果、动作、成本、缓存和Map明细完整。
R6相对Standard/R5的差异能定位到具体request和payload section。
任何压缩策略不得减少node/edge/root/finish coverage。
策略只有在simple零回退、complex有自然激活且收益可归因时才接受。
未自然激活时只确认机制，不宣称live收益。
```

主要收益：把 R5 因零边 Map 得出的压缩 HOLD 结论放回正确模型中重新验证，避免策略叠加误判。

## 1.14 Phase H：正式回归、审查与收口

**目标**：证明设计、实现、日志、测试和文档一致，并形成后续长期 Map 压缩边界。

实施项：

1. deterministic 全矩阵：validator、transitions、mutation、terminal、replay、projection、schema。
2. Docker formal：simple/branch-join/rework/resume/long-map，Standard/R5/R6 各 3 次。
3. 执行代码长度、dead path、forbidden symbol、兼容分支和 provider-visible 文案扫描。
4. 执行 viewer smoke、日志完整性和 observer 报告重放。
5. 获得用户授权后执行独立对抗性审查；发现问题回到对应 Phase 修复并重跑门禁。
6. 产出 R6 closeout、证据索引、保留风险和后续“骨架超限”专项入口。

退出门禁：

```text
correctness/external validation 达到预先冻结门槛，无 runtime 中断污染样本。
所有已提交 Map invariant violations=0，partial commit=0，auto terminal=0。
旧 TaskStatus/MapStatus/root inference/terminal-last-node production symbol=0。
docs、schema、tests、logs、viewer 对 role/status/error code 定义一致。
全部改动已 commit/push，git clean。
```

主要收益：R6 形成可以继续扩展和长期测试的简洁 Map 原语，而不是新的实验性半模型。

## 1.15 实现完整性矩阵

| Plan Item | Production Path | Test Evidence | Runtime Evidence | Phase | Status |
|---|---|---|---|---|---|
| Root/Finish 一等模型 | `action_map/model` | role/status matrix | init digest | B/C | planned |
| 单入口单出口 validator | `action_map/invariants` | graph/property tests | violation codes | B | planned |
| Agent 原子图事务 | control handler + mutation | atomicity/revision tests | mutation trace | C/D | planned |
| Root-open readiness | transitions/runtime | chain/fork/join tests | frontier changes | C/D | planned |
| Agent 手动 Finish | control/runtime | terminal negative matrix | terminal trace | C/E | planned |
| Event reducer/replay | events/snapshot | 20-cycle/fork/crash | hash/revision | B/E | planned |
| 纯 projection | projection/context | coverage/dedup/hash | wire LCP | C/F | planned |
| Viewer DAG | Web Viewer | fixture/screenshot smoke | render digest | D/H | planned |
| Docker 三臂观察 | benchmark harness | 1x/3x matrices | request/cache/map logs | A-G | planned |
| 压缩重基线 | projection/observer | B0/strategy matrix | activation/bytes | G | planned |

## 1.16 日志建设矩阵

| Event | 必需字段 | 禁止内容 |
|---|---|---|
| `graph_validation_started` | map/revision/node/edge counts | 策略评价 |
| `graph_validation_failed` | stable code, node/edge IDs, `state_commit=false` | 下一步建议 |
| `graph_mutation_committed` | old/new revision, exact delta, state hash | Runtime 自动补边 |
| `node_status_changed` | node, role, from/to, source call/event | “已理解”等语义 |
| `readiness_changed` | node, predecessor states | 任务充分性判断 |
| `terminal_requested` | map/revision/finish/source call | 自动总结 |
| `terminal_committed` | root/finish states, summary ref/hash, revision | 改写后 summary |
| `snapshot_replayed` | event range, state hash, mismatch | 静默修复 |
| `projection_rendered` | coverage, bytes, refs, duplicate count | next-action hint |

日志必须能回答“Agent 请求了什么、Runtime 校验了什么、实际提交了什么”，但不能回答“Agent 下一步
应该做什么”。

## 1.17 风险与应对

| Risk | Early Signal | Response |
|---|---|---|
| Phase C 切换面过大 | protocol/runtime/projection 任一出现双权威 | 停止接线，回退整阶段；不加兼容桥 |
| Agent 不自然声明边 | live Map 仍近似 root 星型 | 先查 schema/提示/反馈是否清晰；记录能力缺口，不自动规划 |
| terminal reject loop | 相同 violation 连续出现 | 检查回执是否丢失/扭曲；不得加入语义纠正器 |
| 多前驱 readiness 错误 | 未完成前驱时节点 ready | 纯 transition/property test 阻断发布 |
| snapshot 漂移 | replay hash 不一致 | corruption fatal，修 reducer；不信任 snapshot 覆盖历史 |
| token 增长 | 平均 request input 高于 Standard | 按 wire section 拆解，优先删重复 owner/schema |
| cache 下降 | LCP 提前分叉或同 shape 0-hit | 修结构稳定性和观测口径，不牺牲语义 |
| 压缩伤害全局视野 | node/edge coverage <100% | 拒绝策略并回退 R6-B0 |
| 骨架最终超限 | skeleton-only 超 provider budget | 显式失败，另立 R7/专项，不在 R6 临时分页 |

## 1.18 回退策略

R6 不提供 runtime rollback mode。每个 Phase 的回退单位是其主题 commits：

```text
A/B failure -> 修合同或纯核心，不影响 production。
C failure   -> Git 回退完整纵向接线，production 回到 R5；不保留双写。
D-F failure -> 回退对应主题 commit，保留最近已通过的 R6 模型。
G strategy failure -> 只回退该单一策略，回到 R6-B0/Previous。
H finding   -> 回到归属 Phase 修复，重跑该 Phase 至 H 的受影响门禁。
```

回退后重新生成实验数据，不迁移失败 revision。任何数据修复脚本、legacy reader 或字段猜测都视为
违反无兼容原则。

## 1.19 首个执行动作

用户确认开始 R6 后，只执行 Phase A：

1. 创建机器合同和 ownership/difference inventory；
2. 固定 Standard/R5 Docker 基线；
3. 提交并推送 Phase A 文档/fixture；
4. 暂停汇报盘点结果，必要时调整 B-H 计划；
5. 未经新的推进指令，不进入 Phase B 生产代码。
