# R7 三种 Projection 策略分阶段实施计划

> 本计划从 `00-r7-three-projection-policy-charter.md` 派生。R7 继承 R6 的 canonical Rooted
> DAG、Event Store、工具链和硬约束，只重构 projection 进入 provider context 的策略点。三种策略
> 必须逐个接入同一基建、逐个验证，禁止并行演化三套架构。

> 2026-07-20 后续专项：Phase D.5 是当前生产回滚基线；它删除的是旧版独立 Working Protocol 注入路径。
> 五层重构选择的 L2 不是恢复该路径，而是作为现有 developer bundle 的首个版本化 section，按
> [R7 五层架构可执行规格](25-r7-five-layer-executable-spec.md) 的 FLA-0 至 FLA-3、FLA-3.5、FLA-4 至 FLA-8
> 另行实施。涉及 L1-L5 的
> 目标合同、生产入口和验收以 `25` 号规格及 authority manifest 为准；本计划中的 Phase E-H 继续描述 R7
> projection 主线，不可替代五层专项的完成证据。
> 2026-07-21 更新：FLA-2 的合同 blocker、旧 L4 discriminator、V2 binding 反馈、观测缺口和后续发现的 evidence
> freshness/raw-count gate 缺口均已关闭。当前 Base 2.0.1 / manifest 1.0.2 的简单、复杂 Docker 配对冒烟和四轮
> 独立对抗性闭环已通过，FLA-2 恢复为 `active_verified`，详见
> [阻塞修复结果](30-r7-fla2-blocker-repair-result.md)。H-003 跨 top-level sibling 结构问题仍保持 open，作为后续
> L4 carrier 能力问题独立处理，不得用额外 Runtime 语义干预掩盖。
> 2026-07-21 决策更新：H-003 已确认为连续动作产品合同的结构性回归，不再留到 FLA-6 作为可选实验。
> 新增 [FLA-3.5 连续动作合同回归修复](33-r7-continuous-action-regression-repair-plan.md)，阻塞 FLA-4、
> R7 Phase E 及后续收口。当前 `required_next_call + top-level sibling` 只保留为可复现回归基线，不是目标合同。

## 1.1 元数据

```text
Created: 2026-07-17
Updated: 2026-07-21
Version: v0.0.5 build-R7
Status: Phase D.5 Completed / FLA-3.5 Selected Not Implemented / Phase E Blocked
Owner / Responsible: WhaleCode core runtime / TaskSpace
Risk Level: Critical
Plan Type: Shared architecture with three projection policies
Execution Order: A -> B -> C -> D -> E -> F -> G -> H
Five-Layer Order: FLA-0 -> FLA-1 -> FLA-2 -> FLA-3 -> FLA-3.5 -> FLA-4 -> ... -> FLA-8
R6 Frozen Baseline: e29810158
Compatibility Policy: none
```

## 1.2 执行摘要

```text
A. 冻结合同、盘点 R6 耦合点、固定 Standard/R6 基线
B. 抽取共享 policy 核心，纵向切换 map-always，删除 epoch baseline
C. 在同一核心上接入 map-append 与 supersession 合同
D. 在同一核心上接入 map-request 与共享 read_map action
FLA-3.5. 恢复“非终态交接 + 真实后续动作”同一 Tool schema 的结构保证
E. 收敛 retry/resume/fork/compaction 和跨策略事件等价性
F. 删除残留分叉，完成配置、工具、日志和 Viewer/observer 一致性
G. 执行 Standard + 三策略正式四臂矩阵并冻结默认值建议
H. 全量回归、文档收口；经用户授权后执行对抗性审查
```

每个阶段只引入一个可归因的策略变化。B、C、D 完成后都暂停形成结果文档，不用后续策略掩盖当前
策略的缺陷或成本。

## 1.3 当前代码基线与主要耦合

R6 当前生产路径已知的 projection 相关所有权如下，Phase A 必须逐项补全到函数级 inventory：

| 区域 | 当前职责 | R7 处理 |
|---|---|---|
| `core/src/action_map/projection.rs` | 渲染 `epoch_baseline` projection | 保留为唯一共享 renderer，移除策略字样 |
| `core/src/action_map/runtime.rs` | 构造 bootstrap/active developer context | 只提供 canonical snapshot 与 renderer 输入，不决定注入方式 |
| `core/src/state/taskspace_projection_epoch.rs` | 缓存 anchor、scope、prefix hash | B 阶段删除，由统一 policy cursor 替代 |
| `core/src/state/session.rs` | 保存 projection epoch | 保存单一 `TaskSpaceProjectionPolicy` 与机械 cursor |
| `core/src/session/mod.rs` | 过滤旧 projection、决定 epoch、组合 payload | 收敛为共享 composer + policy decision |
| `core/src/provider_wire_sections.rs` | 假设 active projection 唯一 | 改为 policy-aware 观测，不改变 payload |
| `core/src/client.rs` | 扫描 projection freshness | 按策略校验 always/append/request 合同 |
| `tools/handlers/taskspace_control_*` | Map 控制、expand、output ref | 增加所有策略共用的 `read_map`，其余能力不分叉 |
| `scripts/taskspace-benchmark/` | 成本与 projection observer | 增加 policy、revision 序列和 supersession 指标 |

`MapRuntimeMode` 继续只区分 Standard 与 TaskSpace/Experiment。不得把三个 projection policy 扩展成
三个 Runtime mode；策略必须是独立 enum，并由同一 TaskSpace Runtime 使用。

## 1.4 冻结的工程决策

1. canonical 类型名建议为 `TaskSpaceProjectionPolicy::{MapAlways, MapAppend, MapRequest}`。
2. 只有一个配置字段 `taskspace_projection_policy`；CLI 覆盖、protocol 和 session metadata 都解析为同一 enum。
3. policy 在 session 建立时冻结，Agent 无切换 action，resume/fork 恢复原值。
4. projection renderer 与 policy decision 分离：renderer 只回答“当前 Map 长什么样”，policy 只回答“何时、以何种持久方式放入 context”。
5. `read_map` 是三种策略都可见的共享 control action；`map-request` 不获得专属 tool schema。
6. R6 `epoch baseline + delta journal` 不作为默认、fallback 或 legacy policy 保留。
7. Standard 是 benchmark control，不进入 enum，不受 TaskSpace hard gate。
8. 三种策略长期保留；正式矩阵只决定推荐默认值，不自动淘汰其他策略。
9. 缓存、输入和 Map 显著性属于产品权衡；correctness、反馈保真和状态机不可绕过属于共同硬门禁。
10. 不实现旧 R6 session/snapshot 兼容，不写迁移和双读。

## 1.5 目标模块边界

建议形成以下最小共享结构，最终文件名可在 Phase A inventory 后按现有模块习惯微调：

```text
action_map/projection.rs
  render(snapshot, budget) -> RenderedProjection

action_map/projection_policy.rs
  TaskSpaceProjectionPolicy
  ProjectionTrigger
  ProjectionCursor
  decide(policy, trigger, canonical_identity, cursor) -> ProjectionEmission

session/provider_taskspace_context.rs
  apply_emission(history, projection, emission) -> provider items

tools/handlers/taskspace_control.rs
  read_map -> shared renderer -> exact tool result
```

`ProjectionEmission` 只允许表达 context 机械动作，例如：

```text
None
ReplaceLatest(RenderedProjection)
AppendSnapshot(RenderedProjection)
ReturnAsToolResult(RenderedProjection)
```

不得在 emission 中携带 next action、Agent 建议、节点优先级或 Runtime 生成的语义总结。新的 Whale 自有
代码文件原则上不超过 500 行；不要继续把 policy 逻辑堆入现有大型 `runtime.rs` 或 `session/mod.rs`。

## 1.6 共同门禁

每个代码阶段必须满足：

- 变更前记录根因、预期行为和不应变化的共享能力；
- 变更后执行 unit、integration、replay、provider payload 和 Docker smoke；
- 新增结构化日志，且日志不含 secret、完整用户输入或 Runtime 语义判断；
- 相关代码、测试和文档形成小主题 commit 并 push，不创建新分支；
- 阶段结束时 worktree clean；
- 不保留临时 feature flag、compatibility alias、旧 reader 或 silent fallback；
- live 失败先核对 context/tool feedback 是否丢失、扭曲、重复或过期，再讨论 Agent 行为；
- 代码阶段完成后先汇报，只有用户授权才执行对抗性审查。

## 1.7 横向验证合同

### 1.7.1 阶段快速门禁

Phase B 至 F 每阶段选择 1 至 2 个客观 sample，每个 arm 先运行 1 次：

| Arm | 作用 |
|---|---|
| Standard | 线性上下文成本与 Agent 行为对照 |
| Frozen R6 | 旧 epoch baseline 行为基线 |
| Current R7 | 当前阶段刚接入的策略 |

策略实现阶段不能通过修改 sample prompt 指示 Agent 应何时读 Map、创建多少节点或如何利用缓存。
确定性 fixture 证明机制，live sample 观察自然行为，两者结果分开报告。

### 1.7.2 正式四臂

Phase G 固定四臂：

```text
Standard
R7 map-always
R7 map-append
R7 map-request
```

除 projection policy 外，三个 R7 arm 必须使用同一 binary、commit、Docker image、model、thinking、
temperature、system prompt、tool schema、权限、sample 和 validator。Standard 只关闭 TaskSpace，不修改
任务内容。每个 sample、每个 arm 至少重复 3 次，报告总和、均值和中位数。

### 1.7.3 样本覆盖

- `simple`：单点读取、一次 patch、一次验证，检查固定机制成本和简单任务回归；
- `complex`：多文件调查、实现和验证，观察真实 request 路径；
- `branch-join`：存在自然并行前置与汇合，检查 Map 结构和共享状态机；
- `long-map`：自然长任务，放大 append 累积、always 缓存和 request 读取行为；
- `resume-compaction`：确定性中断、恢复和 context epoch 切换，不用人为提示答案。

确定性测试与本机静态分析可并行。live arms 仅在 provider cache/rate isolation 已证明不会互相污染时
并行；否则按轮换顺序执行，避免为了速度破坏缓存比较。

### 1.7.4 固定指标

结果、动作、成本、缓存和 Map 必须进入同一报告：

```text
correctness / public validator / hidden validator / terminal closure
provider requests / model turns / control calls / ordinary calls / failed calls
wall / provider / tool duration
input / cached input / uncached input / output / total tokens
provider weighted cache hit / request-2+ cache / exact message LCP
projection count / bytes / tokens / positions / revision sequence
same-revision duplicate / stale projection / supersession violations
nodes / edges / depth / indegree / outdegree / frontier / open nodes
read_map calls / revision lag at read / repeated reads / stale_revision errors
semantic retention / rewrite / protected miss / output-ref recovery
```

成本同时记录原始 token 和按当次 provider 官方价格计算的金额；不得把固定“缓存便宜若干倍”写死为
唯一结论。缓存 unavailable 必须明确标记，不能用零冒充。

## 1.8 Phase A：合同、Inventory 与冻结基线

**目标**：在改生产代码前，确认 R6 中所有 projection 所有权、策略耦合和删除路径，冻结 R7 机器合同。

实施项：

1. 创建 `02-r7-phase-a-current-state-inventory.md`，覆盖 renderer、runtime、session、state、protocol、tool、observer、compaction、resume 和 Viewer。
2. 为每个现有 epoch symbol 标记 `retain-shared / replace / delete`，进入 B 前不允许存在 `unknown`。
3. 创建 `r7-projection-policy-contract.json`，冻结三策略 trigger/emission/持久化/freshness/error 矩阵。
4. 冻结配置 enum、非法值错误、session metadata 和 rollout 字段，不先指定最终默认值。
5. 校验冻结的 R6 commit `e29810158`，运行 Standard/R6 的 simple、complex 各 1 次并保存完整 artifacts。
6. 对当前 provider wire trace 离线重算 section、cache 和 projection identity，确认 benchmark 可复用。

退出门禁：

```text
projection ownership inventory 覆盖率 100%。
所有 R6 epoch symbol 都有明确删除或替换阶段。
机器合同覆盖 request/revision/read/compaction/resume 五类 trigger。
Standard/R6 baseline correctness 与 artifact 完整性通过。
本阶段不改变生产行为。
```

收益：防止把 R6 epoch 逻辑遗留为隐藏第四模式，并把后续每项代码改动绑定到可删除的耦合点。

完成证据：

- ownership inventory 覆盖 32 个现存 R6 marker 文件，所有项均有明确 owner 和目标阶段；
- 三策略 trigger、emission、session lifecycle 与共享架构合同已通过机器校验；
- Standard/R6 的 simple、complex Docker 基线均完成并通过 public/hidden validator；
- 四个 arm 的 provider wire 均可离线精确重算 cache、section cost 与 projection identity；
- 结果见 `03-r7-phase-a-result.md`，本阶段没有生产行为变化。

## 1.9 Phase B：共享核心与 `map-always` 纵向切换

**目标**：先建立唯一 policy 核心，并以 `map-always` 完成一次生产纵向切换；同阶段删除 R6 epoch
baseline，不长期双跑。

实施项：

1. 增加 `TaskSpaceProjectionPolicy`、trigger、cursor 和 emission decision 的纯类型/纯函数。
2. 把 `projection.rs` 收敛成策略无关 renderer，移除 `epoch_baseline` 产品语义。
3. provider composer 在最终 wire 前过滤旧 automatic projection，并在自然历史末尾放置一份最新 ephemeral projection。
4. freshness 对账 `map_id + revision + canonical hash + projection hash`。
5. 删除 `taskspace_projection_epoch.rs`、session epoch state、anchor/scope 决策和专属日志。
6. 加入 policy-aware observer；当前只开放 `map-always`，其他 enum 值在接入前不可被生产配置选择。
7. retry、resume、compaction continuation 都重新读取当前 canonical revision。

测试：

- renderer 确定性和同 revision 字节一致；
- 每个 active request 恰好一个 latest projection；
- 历史中旧 automatic projection 为零；
- stale/hash mismatch fixture 必须失败并记录 freshness verdict；
- scripted action sequence 的 Map/event/replay 与 R6 一致；
- Standard/R6/R7-always 对 simple、complex 各 1 次。

退出门禁：

```text
R6 epoch state、anchor 和 scope production symbol 为零。
map-always 所有 request 的 emitted_revision == canonical_revision。
correctness、Root/Finish closure、event replay 和反馈保真 100%。
缓存下降如实归类为策略特征，不能用语义裁剪掩盖。
```

完成证据：

- `TaskSpaceProjectionPolicy`、共享 trigger/cursor/emission decision 已接入生产；
- `map-always` 完成纵向切换，`map-append`、`map-request` 在对应 phase 前机械拒绝；
- R6 epoch state、anchor、scope 与 production marker 已删除，没有兼容或迁移路径；
- simple/complex 共 36 个 TaskSpace provider request 均只有一份 projection，四元 identity 全部匹配；
- 两组 Standard/R7 Docker 样本均通过，冻结 R6 作为历史第三臂进入诊断对照；
- cache 下降已定位为动态 projection 位置破坏精确前缀的 `map-always` 策略特征；
- 结果见 `04-r7-phase-b-result.md`，机器结果见
  `benchmarks/taskspace/r7/phase-b-result.json`。

## 1.10 Phase C：接入 `map-append`

**目标**：复用同一 renderer/composer，只新增“每轮 request 持久追加最新 projection 到 context
末尾”的 emission 规则。

实施项：

1. `ProviderRequest` trigger 机械保证请求末项是当时最新完整 projection，并将新末项持久写入 context。
2. projection 不依赖 control carrier 或 revision commit；Map 未变化时也可在下一轮新历史后再次追加。
3. projection envelope 增加 `projection_kind`、`supersedes_all_prior_projections` 和 `current_state_rule`。
4. cursor 记录末项 projection identity；provider retry 在末项未变化时不重复 emission。
5. provider scanner 校验最后 projection 对齐 canonical Map，历史 revision 非递减。
6. tool schema 和系统提示词使用共享版本规则；不为 append 增加专属 Agent 工作建议。

测试：

- 两轮 request 的 `r -> r` 会在各自末项携带 projection，单次 provider retry 不重复；
- 请求间自然历史和 projection 均只追加，stale revision 返回机械错误；
- 旧 projection 没有永久 current 标记，最后 projection 唯一权威；
- Standard/R6/R7-append 对 simple、long-map 各 1 次。

退出门禁：

```text
request_tail_projection_missing == 0，revision_regression == 0。
last projection identity 与 canonical Map 一致。
消息前缀和 provider cache hit 可由 trace 复核。
输入增长与旧版本数量完整量化，不误报成语义丢失。
```

完成证据：

- `map-append` 已在共享 policy/renderer/composer 上纵向接入，没有新增第二条 context 路径；
- emission 触发点已从 revision/tool result 收敛到 `ProviderRequest`，每轮持久追加 request-tail snapshot；
- projection 使用自然历史兼容的 `user` carrier，DeepSeek wire 不再产生 interleaved `system`；
- simple/complex 共 31 个 TaskSpace provider request，末项、identity、revision 非递减与 scanner 均
  31/31 通过；同 revision 跨 request 重复被正确保留；
- 两组 Docker Standard/R7 均 solved；R7 request 2+ cache hit 从旧实现的 46.51%/69.36% 提升到
  78.95%/87.35%，same-shape zero hit 均为零；
- 旧 projection 累积造成的 input 增长继续作为 `map-append` 已知产品成本记录，不包装为缓存 bug；
- 结果见 `05-r7-phase-c-result.md`，机器结果见
  `benchmarks/taskspace/r7/phase-c-result.json`，根因闭环见
  `coe/2026-07-18-06-36-r7-map-append-cache-gap.md`。

Phase C 后续消融移除了 bootstrap/terminal 命名 `tool_choice` 和命名工具自动关闭 thinking 的隐式
耦合。简单、复杂各 3 次 Docker 运行均 solved，92 个 provider payload 均保持 `auto`，首请求 thinking
均有效；但 6/6 都在普通工具收到 `no_task_path` 后才于第二次请求初始化 Map，首请求初始化为 0/6。
在明确接受“稳定晚一轮”还是要求“首轮初始化”之前，Phase D 暂不把该行为视为已收口。结果见
`06-r7-tool-choice-ablation-result.md` 和
`benchmarks/taskspace/r7/phase-c-tool-choice-ablation-result.json`。

后续静态 bootstrap 合同实验没有恢复动态 `tool_choice`：`taskspace_control` 永久置顶，固定 schema
明确首动作合同，bootstrap projection 只暴露机械硬状态。简单、复杂各 3 次均在首请求初始化，合计
从 0/6 提升为 6/6；64 个 TaskSpace payload 全部保持 `auto`、同一 13-tool hash、零 shape transition，
两组均 solved。Phase D 准入恢复，结果见 `07-r7-static-bootstrap-contract-result.md` 和
`benchmarks/taskspace/r7/phase-c-static-bootstrap-contract-result.json`。

## 1.11 Phase D：接入 `map-request`

**目标**：普通 request 不自动带完整 projection，同时保持 TaskSpace 状态机硬约束不可绕过。

实施项：

1. 在共享 `taskspace_control` 增加 `read_map` action，三种策略的 tool schema 完全相同。
2. `read_map` 调用同一 renderer，tool result 返回当前 map id/revision/hash 和完整 projection/ref。
3. `map-request` 对普通 request 返回 `Emission::None`；显式读取返回 `ReturnAsToolResult`。
4. 空 Map、无有效 binding、未终结 Root/Finish 的现有硬 gate 保持不变并补足回归测试。
5. 初始/恢复 context 只提供机械 Map handle；不自动读 Map、不规定读取频率、不增加提醒循环。
6. observer 记录读取请求、完成、revision lag 和重复读取，不评价 Agent 是否“应该读”。

测试：

- ordinary request 自动 projection count 为零；
- `read_map` 与同 revision always/append renderer 输出 hash 一致；
- 未初始化普通工具、无 lease 普通工具、未 finish 结束均被原硬规则拒绝；
- 合法但未读取 Map 的 ordinary action 不因策略被额外拒绝；
- Standard/R6/R7-request 对 simple、complex 各 1 次。

退出门禁：

```text
automatic full projection count == 0。
Agent 发起的 read_map 结果 100% 对齐 canonical revision/hash。
TaskSpace bypass fixture 100% rejected，合法动作无新增 policy rejection。
不出现 Runtime 自动 read、自动提醒或自动纠正事件。
```

完成证据：

- 共享 `taskspace_control.read_map`、renderer、policy decision 和 map handle 已接入，无策略专属工具 schema；
- simple/complex 共 30 个 R7-request provider payload，automatic projection count 均为 0；
- simple 由 Agent 显式读取 1 次，revision/hash 对齐 canonical，随后正常闭合；
- complex 在完全未读取 Map 的情况下，初始化后执行 32 次合法 ordinary tool，无新增 policy rejection；
- simple Standard/R7 均 solved，R7 相对 Frozen R6 request/input/uncached input 明显下降；
- complex 修复和 8/8 validator 通过，但 Agent 未闭合 Map 就输出 plain final，既有 R6 no-retry terminal
  gate 产生 protocol violation；该负向产品证据不归因成 Phase D emission bug；
- Phase E 不自动启动。结果见 `08-r7-phase-d-result.md`，机器结果见
  `benchmarks/taskspace/r7/phase-d-result.json`，失败链见
  `coe/2026-07-19-04-58-r7-map-request-complex-interruption.md`。

### 1.11.1 Phase D.1：内置核心工作协议

**目标**：在不扩大 Runtime 责任、不污染 projection 的前提下，让 Agent 明确知道 TaskSpace Map 的工作方法。

完成项：

1. 增加三种 projection policy 共用的静态 TaskSpace developer 协议；Standard 零注入。
2. 协议作为每次最终 provider input 的固定首项构造，不写入自然历史，不随 Map 状态动态变化。
3. 协议声明 Agent 的 bootstrap、阶段维护、按需读 Map 和显式终局职责；Runtime 仍只负责硬规则。
4. 建立 `schema_version + protocol_version + rules_sha256` 版本身份和可执行合同。
5. provider wire trace v4 与性能报告逐请求记录版本、哈希、位置、角色和 estimated tokens。
6. Docker simple/complex 分别验证 `v1.0.0`、`v1.0.1`，每轮均有同期 Standard。

结果：

- `v1.0.0`、`v1.0.1` 的两个 complex `map-request` 均完整闭合并 solved；Phase D 未闭合症状未复现；
- `v1.0.1` 两个 TaskSpace run 都首工具初始化，22/22 请求协议身份匹配，Standard 19/19 零注入；
- `v1.0.1` simple 为 7 vs 9 requests，complex 为 12 vs 13 requests；两组 ordinary tools 与 Standard 相同；
- same-response lifecycle batching 指令没有产生 multiple control response，两组仍各有 3 次 standalone transition；
- 不继续增加提示词压力。后续若解决该 cadence，应设计三策略共享、由 Agent 显式声明的组合 tool shape；
  Runtime 只机械校验和执行；
- `v1.0.1` 固定成本约 431 estimated tokens/request，后续文本压缩必须作为独立版本实验。

证据：

- `09-r7-working-protocol-v1-result.md`
- `10-r7-working-protocol-v1-0-1-result.md`
- `benchmarks/taskspace/r7/working-protocol-contract.json`（历史协议合同，已由 D.5 替代）
- `benchmarks/taskspace/r7/working-protocol-v1.0.0-result.json`
- `benchmarks/taskspace/r7/working-protocol-v1.0.1-result.json`

### 1.11.2 Phase D.2：原子完成交接合同

**目标**：修复 R6/R7 将 Work 完成和下一节点绑定重新拆成独立 control 调用的回归，让工具合同与工作协议一致，
同时不让 Runtime 推断 Agent 的下一步。

完成项：

1. 从 provider 可见的 `transition_node` schema 移除独立 `complete`；内部状态机仍保留完成原语。
2. 在共享 `taskspace_control` 增加 `complete_then_continue` 与 `complete_then_end`。
3. `complete_then_continue` 要求 Agent 显式给出当前节点、下一节点和 continuation；完成、readiness、bind 与
   后续普通动作在一个 control transaction 中执行。
4. `complete_then_end` 要求 Agent 显式给出最终总结；完成当前 Work、闭合 Finish 和 Root 在一个 revision 中提交。
5. candidate graph、lease 和 terminal persistence 均保持全成或全不成；失败结果固定
   `state_commit=false/partial_commit=0`，Runtime 不修复畸形 JSON、不猜测下一节点。
6. replay、control feedback、working protocol `v1.0.2`、日志和性能 observer 同步识别两种新 action。

验证结果：

- Rust 相关单测、集成测试、observer 自测、K0 自测和 skill 校验通过；
- Docker simple/complex 均与同期 Standard 一起 solved，公开与隐藏验证均通过；
- 两个 TaskSpace run 都采用 `complete_then_continue=1`、`complete_then_end=1`，
  `standalone complete=0`、`finish_end=0`；
- simple 请求数 8 vs Standard 7，complex 为 12 vs 10；没有再由节点完成本身产生独立 provider request；
- complex 首次大型嵌套 patch carrier 发生一次 trailing JSON 和一次空参数调用，Runtime 忠实拒绝且零提交，
  Agent 随后自行恢复。该生成稳定性观察与 completion handoff 回归分开记录，不通过 Runtime 语义修补处理。

证据：

- `11-r7-atomic-completion-handoff-result.md`
- `benchmarks/taskspace/r7/working-protocol-v1.0.2-result.json`
- `coe/2026-07-16-18-52-r6-phase-f-context-cost.md` 的 H-009 / E-022
- implementation commit `26814f3f4`

Phase D.2 的 completion action 决策点已经关闭；其共用嵌套 patch carrier 在 D.3 被确认为待修合同。
Phase E 仍不得改变三种 projection policy 的状态机、事件或工具集合等价性。

### 1.11.3 Phase D.3：嵌套 Patch Carrier 根因诊断

**目标**：定位 complex 样本中大型 patch control 参数 trailing JSON 与后续空参数调用的真实产生层，
避免以 Runtime 容错或 projection 注入掩盖 provider/tool contract 缺陷。

诊断完成项：

1. 对 54 次历史真实 `patch_then_actions` carrier 统计失败分布；15 次 JSON 非法，问题跨
   `transition_node(bind)` 与 `complete_then_continue` 存在。
2. 使用生产 schema 建立非流式六臂 provider probe，排除 Whale SSE assembler。
3. 逐字节对账 Runtime failure 与后续 provider payload，排除 feedback 丢失、改写和 call id 错配。
4. 证明减少嵌套或把 `patch_input` 提升到浅层只能改善 JSON 合法率，不能保证 patch 正文保真。
5. 证明独立 `apply_patch` 是当前唯一达到 6/6 JSON 合法与 6/6 正文一致的形态。

当前判定：

- Runtime 严格拒绝和零提交语义正确，不得修 JSON、猜 patch 或自动推进状态；
- 共用嵌套 patch carrier 是根因所在的产品合同，不能只修 completion action；
- 优先修复候选为“同一 response 的小型 lifecycle control + direct patch sibling”，但该候选尚需
  provider 双调用、sequence preflight 和真实 Docker 验证，不能以诊断 probe 代替生产验收；
- Phase E 保持未启动，先关闭 D.3 的生产修复门。

证据：

- `12-r7-nested-patch-control-root-cause.md`
- `13-r7-lean-response-sequence-repair-plan.md`
- `benchmarks/taskspace/r7/nested-patch-control-probe-result.json`
- `coe/2026-07-19-19-30-r7-nested-patch-control-arguments.md`

### 1.11.4 Phase D.4：顶层 Patch 修复与采用率验证

已完成：

1. 删除产品 control 中的 nested ordinary/patch payload，不保留兼容 parser。
2. 使用 `required_next_call=ordinary_tool|apply_patch` 声明紧邻顶层 sibling；该字段只声明，不执行或安排。
3. 增加 full-response preflight、control/patch barrier 和原生顶层 patch 反馈。
4. 升级 working protocol `v1.0.4`、observer、provider probe 和 Docker 对照证据。

验收结果：

- provider probe 6/6 生成 `control -> patch`，6/6 patch exact；合并 request 未被破坏；
- simple/complex 与同期 Standard 都 solved，公开和隐藏验证通过，Map 全部闭合；
- 两个自然 TaskSpace 样本仍各有 2 次首次 sibling 遗漏，收到明确 preflight 失败后才在下一请求纠正；
- complex 另有错误 bind、patch 上下文失败、三 patch 同响应和过早 terminal 等独立 Agent 行为，不能混算为
  patch carrier 回归。

因此 Phase D.4 的 patch fidelity 与执行边界通过，但首次采用率/request 效率门禁未通过。Phase E 暂不启动，
先单独收敛 provider-visible sibling 调用协议；禁止 Runtime 自动补调用或恢复 nested carrier。

证据：

- `14-r7-required-next-call-validation-result.md`
- `benchmarks/taskspace/r7/working-protocol-v1.0.3-result.json`
- `benchmarks/taskspace/r7/working-protocol-v1.0.4-result.json`
- implementation commit `12e7f8e3e`
- observer commit `04ac1ba24`

### 1.11.5 Phase D.5：双基础提示词收敛

**目标**：停止用极简 Whale base 加额外 TaskSpace developer protocol 拼接行为框架，改为按会话工作方式选择
一份完整、成熟且内部一致的 `base_instructions`。

实施项：

1. Standard 使用继承 Codex 原生框架的 WhaleCode 完整 base，只修改产品品牌暴露。
2. TaskSpace 使用同一 Codex 写法和章节框架，把 Planning 有机替换为 TaskSpace Map 工作方法。
3. 删除独立 Working Protocol developer message 及其注入、去重和旧身份观察路径。
4. 每次 provider request 从同一状态快照同时确定 base profile 与计划工具可见性：Standard 只见
   `update_plan`，TaskSpace 只见 `taskspace_control`。
5. 启动预热在 resume 完成后按实际模式构建；空闲态模式切换后取消旧预热并按新 profile 重建。
6. 两份 base 独立版本化和哈希；provider wire trace v5 逐请求记录 profile、version、hash、位置和成本。
7. 三种 projection policy 继续共享同一 TaskSpace base，不产生 policy 专属提示词或 Runtime 分叉。

当前状态：实现、合同与单组 Docker 诊断对照均完成。Standard/TaskSpace 都通过公开及隐藏验证，线上
base identity 分别 6/6、7/7 匹配 v1.0.0 合同。该单次结果只证明接线正确，不估计总体收益。详细设计与
结果见 `20-r7-dual-base-instructions-design.md`、`21-r7-dual-base-instructions-result.md`，机器证据见
`benchmarks/taskspace/r7/base-instructions-contract.json` 与
`benchmarks/taskspace/r7/dual-base-instructions-v1.0.0-result.json`。

### 1.11.6 FLA-3.5：连续动作合同回归修复

**目标**：恢复 R5 J6 与 R7 D.2 已验证的结构保证，使初始化、绑定和完成后继续不能脱离真实后续动作单独
表达，同时保留 D.4 的原生 Patch 保真收益。

选定方向是让真实动作 Tool 由共享 builder 机械增加轻量 `taskspace_transition` 前缀；状态交接和该动作属于
同一个 provider tool call，Patch 正文仍保持原生顶层输入。Runtime 只校验并执行 Agent 明确给出的交接，
不自动补动作、不推断下一节点，也不复制 ordinary Tool router/handler。

实施必须先完成真实 provider、`apply_patch`、MCP、反馈保真和 barrier probe。probe 未满足 100% 结构合法、
Patch exact 与输入输出保真时不得进入生产。完整 CA-0 至 CA-6、三臂验收、日志、回滚和冲突处理见
[连续动作合同回归修复计划](33-r7-continuous-action-regression-repair-plan.md)。

阶段关系：

- 当前 sibling 方案是回归基线，不再由 FLA-4 正式化；
- FLA-3.5 未 `active_verified` 前，FLA-4、Phase E 及后续阶段保持阻塞；
- 原 FLA-6 “移除 `required_next_call`”实验取消，该字段随回归修复一次性删除；
- 历史 D.2-D.4 结果保持原样，继续分别证明连续动作收益和 Patch carrier 根因。

## 1.12 Phase E：生命周期与跨策略等价

**目标**：证明三种策略只改变 provider context projection，不改变任何状态机、工具或事件结果。

实施项：

1. 建立相同 scripted action sequence 的三策略 differential test。
2. 对比 canonical event bytes/hash、Map revision、lease、ordinary tool dispatch 和 terminal summary。
3. 覆盖 retry、provider error、tool error、resume、fork、rollback 和 compaction。
4. 实现并验证新 context epoch 规则：always 注入最新、append 追加当前起点、request 只给 Map handle。
5. compaction 只处理 provider history，canonical Map/Event Store hash 必须保持。
6. subagent 继承 Map/node/lease 和 policy；不得因 policy 改变工具权限。

退出门禁：

```text
同一合法 action sequence 的 canonical Map/event/terminal hash 三策略完全一致。
唯一允许的 diff 位于 provider context projection items 与对应 observer events。
resume/fork/compaction 后 policy、revision 和 state hash 精确恢复。
simple + resume-compaction 快速三臂门禁通过。
```

## 1.13 Phase F：单架构审计与产品面收敛

**目标**：删除所有临时接线和策略泄漏，使配置、工具、Runtime、observer、Viewer 只依赖一个共享合同。

实施项：

1. 静态扫描 mode-specific Runtime/handler/renderer/schema，任何复制分支都必须删除或证明只是 emission decision。
2. 三策略 `tools_hash`、tool count、system prompt 主体和 ordinary permissions 必须相同。
3. CLI/config/protocol/session/rollout/Viewer 统一显示 canonical policy 名称。
4. provider wire scanner 不再把 always 的唯一性断言误用到 append/request。
5. benchmark skill/report 固定输出四臂表、request 明细、cache、projection 和 Map 指标。
6. 运行所有 TaskSpace unit/integration/replay、benchmark harness self-test 和 Docker smoke。

退出门禁：

```text
除 projection emission decision 外无 policy-specific 执行分支。
三策略 tools_hash/system-prompt-hash/permission digest 相同。
旧 R6 epoch marker/state/log/schema 扫描结果为零。
observer 对缺失数据明确 unavailable，不产生误判。
```

## 1.14 Phase G：正式四臂矩阵

**目标**：在同一生产 commit 上量化三种设计取舍，形成默认值建议，但保留全部策略。

执行：

1. 构建唯一 R7 Docker image，参数化运行三策略；Standard 使用同 image 的 Standard mode。
2. simple、complex、branch-join、long-map 每 arm 至少 3 次，运行顺序轮换。
3. 对失败 side 先完成 trace、feedback、projection freshness 和 validator 分析，不能直接剔除。
4. 分别报告 correctness、请求路径、输入/缓存/金额、Map 质量、读取行为和陈旧上下文影响。
5. 把每个策略的已知特征与实现 bug 分栏，不用一个综合分数隐藏取舍。
6. 基于证据提出推荐默认值；三种参数继续可用，不因推荐默认值删除其他策略。

决策维度：

| 维度 | map-always | map-append | map-request |
|---|---|---|---|
| 当前全景持续可见 | 预期最高，需实测 | 高，但含旧版本 | 取决于 Agent 主动读取 |
| 自动缓存适配 | 预期最低，需实测 | 预期最高 | 接近线性 Standard，需实测 |
| 总 input 增长 | 当前 projection 重复计费 | 随 revision 累积 | 取决于 read_map 次数 |
| 陈旧状态干扰 | 最低 | 预期最高，需 trace 验证 | 取决于读取历史 |
| Map 工作约束 | 相同 | 相同 | 相同且必须不可绕过 |

退出门禁：

```text
所有有效 side correctness/terminal/Map invariant 通过。
失败和 outlier 有逐 request 根因，不静默剔除。
每项结论可追溯到 artifact、commit、image 和 policy。
默认值建议建立在总和/均值/中位数和行为质量上，而非单样本直觉。
```

## 1.15 Phase H：收口与经授权审查

**目标**：完成 R7 发布判定、文档同步和残余风险说明。

实施项：

1. 全量 Rust test、type/build、PowerShell benchmark self-test、Docker smoke 和 replay proof。
2. 更新 R7 章程/计划状态、Phase 结果、配置说明、运行与日志获取经验。
3. 检查 secret、artifact、`.env.local` 和 provider payload 不进入 Git。
4. 检查 worktree clean、提交已 push、R6 文档保持历史原样。
5. 用户明确授权后，使用独立 reviewer 执行对抗性审查；未授权不执行。

完成标准：R7 总验收 10 项全部有代码、测试、日志或 benchmark 证据，且没有未解释的架构分叉或
兼容债。

## 1.16 风险与控制

| 风险 | 早期信号 | 控制 |
|---|---|---|
| policy 泄漏到 Runtime 语义 | 不同策略产生不同 Map/event hash | Phase E differential hard gate |
| append 缺失或倒退 | request 末项不是 projection、revision 回退、末项 identity 不匹配 | cursor + wire scanner + fault tests |
| request 退化成可选账本 | ordinary tool 可在空 Map/无 lease 执行 | 共享 hard gate 回归矩阵 |
| 连续动作退化成跨 sibling 事后惩罚 | standalone transition、`TASKSPACE_REQUIRED_SIBLING_MISSING` | FLA-3.5 schema-first hard gate |
| always 缓存被误判为 bug | correctness 正常但 uncached 偏高 | known-feature 分类 + raw cache trace |
| mode-specific prompt 污染实验 | 三策略 system/tool hash 不同 | Phase F hash equality gate |
| compaction 丢失 Map | provider history 缩短后 state hash 变化 | canonical store 独立 hash proof |
| 旧 R6 路径成为隐藏第四模式 | epoch symbol 或 marker 仍生产可达 | Phase B 删除 + Phase F static audit |
| live 并行污染缓存结论 | 命中率随执行顺序异常漂移 | 隔离证明前顺序轮换，不盲目并行 |

## 1.17 不采用方案

1. **三个独立 Runtime mode**：会把工具、状态机和反馈链分叉，无法把结果归因于 projection。
2. **保留 R6 epoch baseline 作为 fallback**：形成第四种隐藏语义和长期兼容债。
3. **map-request 取消硬 gate**：会把 TaskSpace 降为可绕过的普通 ledger tool，违背产品定义。
4. **append 自动删除旧 projection**：破坏线性追加定义；历史清理由共享 compaction 负责。
5. **always 通过语义压缩换缓存**：混入第二变量，无法判断全景替换策略本身的收益和成本。
6. **按策略修改提示词或 tool schema**：会让实验比较同时受到 Agent 引导差异影响。
7. **正式矩阵前选默认赢家**：当前只有机制推断和 R6 历史证据，尚无同 commit 四臂数据。

## 1.18 外部依据

1. [DeepSeek Context Caching](https://api-docs.deepseek.com/guides/kv_cache/)：缓存按完整前缀单元
   匹配，支持把三策略的缓存差异作为可测量输入，而不是主观假设。
2. [DeepSeek Anthropic API compatibility](https://api-docs.deepseek.com/guides/anthropic_api/)：当前
   `cache_control` 被忽略，因此计划不依赖用户标记 cache unit。
3. [JSON Schema conditional validation](https://json-schema.org/understanding-json-schema/reference/conditionals)：
   用互斥 schema 表达机械配置与 action 合同，不复制三套工具定义。
4. [Temporal history service architecture](https://github.com/temporalio/temporal/blob/main/docs/architecture/history-service.md)：
   projection 可由事件恢复，不承担 canonical state 所有权。
