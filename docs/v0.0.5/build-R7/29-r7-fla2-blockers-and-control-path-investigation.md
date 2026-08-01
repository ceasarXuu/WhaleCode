# R7 FLA-2 阻塞与控制路径调查

- 日期：2026-07-20
- 状态：`repaired / adversarially_reaccepted / H-003 open`
- 调查范围：FLA-2 的 L1/L2 有效性、TaskSpace 可疑执行路径与 benchmark 观测口径
- 生产行为：本轮未修改
- COE：[`2026-07-20-21-24-r7-fla2-control-path-observability.md`](../../../coe/2026-07-20-21-24-r7-fla2-control-path-observability.md)
- 对抗审查：[`2026-07-20-r7-fla2-l1-l2-effectiveness-review.md`](../../../vs_review/2026-07-20-r7-fla2-l1-l2-effectiveness-review.md)
- 修复结果：[`30-r7-fla2-blocker-repair-result.md`](30-r7-fla2-blocker-repair-result.md)

## 1. 历史验收结论

FLA-2 的生产装配已经生效，六次 TaskSpace 样本也都完成了任务；但 `active_verified` 结论不成立。两个合同级
blocker 已通过请求级和代码级证据门，观测产物又系统性少报 preflight reject 与真实 state commit。因此，当时的
结论为：

1. FLA-2 改为 `acceptance_blocked`，不能作为 FLA-3 的已验证前置阶段。
2. 既有正确性、成本和缓存数据仍可保留为行为样本，不得用来证明五层 wire 合同完整或 L1/L2 提升了协议遵循。
3. 本轮只记录和诊断，不修改 composer、Tool、Runtime 或观察器。
4. `25` 号规格和 authority manifest 中的 `active_verified` 是待修正的旧验收声明；在 blocker 关闭前，以本调查和
   `28` 号结果的阻塞标记为执行暂停依据。authority/hash 链的同步属于后续修复提交，不能在诊断提交中伪装完成。

## 2. 两个 Blocker

| ID | 表现 | 已确认直接原因 | 影响 |
|---|---|---|---|
| B1 | TaskSpace provider 请求有第三条 405-byte `system`，与冻结的“两条 system”合同冲突 | `build_initial_context` 把 `map-request` handle 装配成独立 developer message；它只按初始状态生成，后续请求持续保留 | Map 提交后仍同时看到 `map_id:none/bootstrap_required:true`；wire 合同和 canonical 状态一致性均失败 |
| B2 | L2 要求根据 action、submitted、canonical、revision、`state_commit` 恢复，但实际拒绝结果没有统一提供 | FLA-2 提前激活了依赖 FLA-5 Result V2 的说明；现行 preflight、ordinary gate 和 R6 handler 使用三套不同 envelope | Agent 无法按 L2 所述机械对账；L2 与 Tool/Runtime 的能力合同自相矛盾 |

### 2.1 B1 证据链

- 六次 TaskSpace 首请求都有三个 system message，字节数固定为 `19247 / 2014 / 405`，三条 hash 完全相同。
- 65 个 TaskSpace 请求中，所有后续请求都保留包含第三条消息的初始前缀。
- 初始 handle 声明 `map_id:none, revision:none, bootstrap_required:true`；同一 rollout 后续已成功提交
  `map-1/revision=2`，但第三条消息未变化。
- [`session/mod.rs`](../../../third_party/codex-cli/codex-rs/core/src/session/mod.rs) 只在 initial context 构造 handle；
  steady-state 路径只处理 settings diff。
- 现有合同测试只检查 L2 是 developer bundle 第一段且唯一，没有断言完整 provider message 数量；wire observer 也只
  由已识别的 Base identity 推导 manifest identity，所以能在存在第三条 system 时错误通过。

B1 没有造成本轮初始化重试：所有重复 initialize 都发生在首次成功提交之前。它是确定的合同和状态一致性缺陷，
不是首轮 ordinary-before-init 的原因。

### 2.2 B2 证据链

| 失败阶段 | 当前 schema | Agent 可见事实 | L2 要求字段是否齐全 |
|---|---|---|---|
| sequence preflight | `ToolSequencePreflightResultV1` | status、success、error、request counts | 否 |
| ordinary-tool gate | `TaskSpaceGateResultV1` | error、blocking items、missing evidence | 否 |
| control handler | `TaskSpaceControlResultR6V1` | status、revision、violations、commit flag 等部分字段 | 否，缺少统一 action/submitted/canonical/actual/expected |
| FLA-5 目标 | `TaskSpaceControlResultV2` | 统一 factual result algebra | 尚未进入生产 |

这里不是 benchmark 采集丢字段：缺失发生在 Agent 实际收到的 Tool result 中。直接回归窗口是 FLA-2 把 L2 标成
active，而它描述的结果能力仍位于未实施的 FLA-5。

## 3. 六次 TaskSpace 执行路径

左右臂由每个 pair 的 `logical-mode-map.json` 确定，以下只统计真实 TaskSpace side。

| 样本 / repeat | 首动作 | Control | Init 尝试 / 提交 | Missing sibling | No path | Invalid args | State reject | `read_map` |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| simple / 1 | ordinary | 6 | 2 / 4 | 1 | 2 | 1 | 0 | 0 |
| simple / 2 | ordinary | 8 | 2 / 4 | 2 | 2 | 0 | 1 | 1 |
| simple / 3 | taskspace | 6 | 2 / 4 | 1 | 0 | 1 | 0 | 0 |
| complex / 1 | ordinary | 7 | 2 / 4 | 2 | 2 | 1 | 0 | 0 |
| complex / 2 | taskspace | 10 | 3 / 5 | 3 | 0 | 0 | 1 | 1 |
| complex / 3 | ordinary | 8 | 2 / 4 | 2 | 1 | 2 | 0 | 0 |
| **总计** | ordinary 4/6 | **45** | **13 / 25** | **11** | **7** | **5** | **2** | **2** |

### 3.1 初始化前普通工具

这是 Agent 在完整上下文下的动作选择，不是语义传递丢失：

- 六次首请求都完整包含 L1、L2、空 Map handle 和 TaskSpace Tool schema。
- 4 次异常首轮 reasoning 明确表达先探索 workspace、README 或 tests，provider response 本身只生成普通工具。
- Runtime 没有丢弃已生成的 control；收到 `no_task_path` 后 Agent 都能识别要先初始化。
- Chat adapter 按 tool-call index 保留全部调用，后续合法 control+sibling 和普通并行工具也证明链路具备多调用能力。

因此不能把该行为归因于 context 丢失，也没有证据推断更细的模型内部“动机”。空 Map gate 是符合边界的机械硬约束。

### 3.2 单独 initialize / complete

11 次 preflight reject 都是：control 参数已经填写 `required_next_call`，同一 provider response 却没有真实 top-level
sibling。现行 Tool schema 的文字已经明确“声明不等于执行”，但 JSON Schema 只能约束 control 对象内部字段，无法
结构性要求 provider response 中另有 sibling call。Agent 每次都在明确 preflight 反馈后纠正。

这是跨工具序列合同的结构表达缺口，不宜继续通过追加提示词或 Runtime 语义纠正处理。

### 3.3 非法 lifecycle 参数

FLA-2 有 4 次 provider 原始调用使用：

```json
{"action":"transition_node","transition":"complete_then_continue"}
```

现行嵌套 `transition` 只接受 bind/block/unblock/rework。Agent 把目标 lifecycle operation 混入旧 discriminator；参数在
provider output 中已经错误，不是 adapter、projection 或反馈层改坏。该原因与既定 FLA-4“删除
`transition_node + transition`，把 lifecycle operation 提升为直接 action”一致。

### 3.4 两次 state reject

- simple repeat 2：initialize 成功后冗余 bind 已运行节点，状态机正确拒绝。成功结果未直接给出 active node id，薄拒绝
  又只给 `transition_invalid`，Agent 随后 `read_map` 才确认状态。反馈缺口与冗余读取有关联，但 2/6 样本不足以证明因果。
- complex repeat 2：Agent 建图为 `root -> explore -> diagnose -> fix -> verify`，完成 explore 后试图直接跳到 fix，跳过
  自己创建的 diagnose。状态机拒绝依赖违规是正确行为；Agent 随后读取 Map、绑定 diagnose 并恢复。这里的首要问题是
  Agent 工作路径与自建 Map 不一致，不能通过放宽 Runtime 硬约束解决。

## 4. FLA-0 与 FLA-2 行为对照

| 版本 | Control | Init 尝试 | 有效提交 | Missing sibling | No path | Invalid args | State reject | `read_map` | Raw failures |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| FLA-0 | 45 | 14 | 24 | 11 | 6 | 3 | 4 | 3 | 24 |
| FLA-2 | 45 | 13 | 25 | 11 | 7 | 5 | 2 | 2 | 25 |

两个版本的 missing sibling 完全相同，总 raw failure 也没有下降。首次普通工具从 5/6 变为 4/6，样本量和随机性
不足以归因。FLA-2 已证明 L1/L2 装配和固定字节成本变化，但没有证明生命周期协议遵循收益。

## 5. 观测缺口

| 产物 / 字段 | 名称给人的含义 | 实际采集口径 | 分类 |
|---|---|---|---|
| `control_failure_count=7` | 所有 control 失败 | 只数 `TaskSpaceControlResultV1/V2/R6V1 success=false` | 真实缺口：漏掉 11 个 control preflight reject |
| ordinary gate | TaskSpace 协议失败的一部分 | 因 call id 不是 control 而排除 | handler 指标下合理，但缺少统一 protocol failure 总量 |
| `state_commit_count=0` | 已提交状态变更 | 只数 action 名字面等于 `state_commit` | 指标实现/命名错误：现行动作没有该名字，漏掉 25 次 `state_commit=true` |
| `runtime_state_commit_count=0` | Runtime 提交数 | 只数 observability timeline 的 `updateKind=state_commit*` | 窄数据源合理，但名称不能代表 raw committed results |
| `failure-taxonomy-summary.json` 为空 | 所有原始失败分类 | 只聚合 pair outcome taxonomy；六个 pair 都 solved | 聚合口径合理，文件名和使用方式容易误导 |
| manifest identity verified | 完整五层 wire 合同通过 | 由 Base identity 推导，未验证字面 manifest 和 system 总数 | 真实合同观测缺口 |
| `active_projection_missing` | projection 异常缺失 | `map-request` 按合同本就不直接注入 active projection | 名称误导，不能作为失败 |

应当同时保留五种互不替代的计数：sequence preflight、control handler、ordinary gate、committed mutation 和 pair
outcome。汇总层只能组合这些事实，不能用一个 `control_failure` 或 `failure taxonomy` 名称覆盖不同阶段。

## 6. 已确认与待验证

已确认：

1. B1 第三条静态 handle 是 composer 所有权和完整 wire 合同测试缺口。
2. B2 是 L2 提前依赖未实施 L5 的阶段合同错位。
3. missing sibling 是跨 top-level Tool call 约束无法由当前 schema 结构承载。
4. 非法 `complete_then_continue` 嵌套值来自旧 L4 discriminator。
5. ordinary-before-init 不是上下文或 adapter 丢失。
6. benchmark 少报由确定的 parser 和指标口径产生。

尚未确认：initialize 成功反馈缺少 active binding 与后续 redundant bind/read 的因果强度。现有证据只有相关性，后续应
在统一 factual result 的单变量实验中验证，不能据此增加 Runtime 行为约束。

## 7. 后续入口

修复入口已经由用户批准并完成实施，结果见 30 号文档。原诊断的 B1、B2、H-004、H-006 和 H-007 反馈机制均已
进入生产并通过真实请求验证；H-003 仍作为独立结构问题保留。

## 8. 2026-07-21 修复回填

| 原问题 | 修复 | 请求级结果 |
|---|---|---|
| B1 第三条静态 system handle | 每次请求从 canonical 状态构造 user-tail handle | 当前 21/21 TaskSpace 请求只有两条 system，handle 均唯一且位于末尾；17/17 Standard 零注入 |
| B2 L2/Result 能力错位 | L2 升级至 v2.1，control 统一使用 Result V2 | 当前 13/13 control 输出为 V2，5 个拒绝均明确 `state_commit=false` |
| H-004 观测少报 | 拆分 preflight/handler/gate/commit 指标并支持 V2 lineage | 当前 13 control = 8 commit + 5 preflight；8 graph commit 与 8 state commit 对齐 |
| H-006 旧 discriminator | lifecycle 和状态变更改为直接 action | nested transition 和非法 lifecycle 参数均为 0 |
| H-007 binding 事实缺失 | initialize 结果增加独立 `node_bound` step | 2/2 初始化提交包含 binding；两次运行均无 redundant bind/read |

H-003 没有被伪装关闭。current-identity 新样本仍有 5 次 standalone control 被 preflight 原子拒绝，说明文字合同和事实反馈已经
正确，但当前函数调用 JSON Schema 无法结构性要求另一个 top-level sibling。该问题需要独立 Tool 交互形状实验，
不能通过 Runtime 代替 Agent 选动作或追加语义纠正解决。

## 9. 最终复验

Round 2 发现正式 smoke identity 晚于当前 Base/manifest，Round 3 又发现 freshness gate 没有反查机器结果计数；两项
均作为证据链 blocker 接受并修复。Round 4 独立 reviewer 重跑自测和真实 gate，并额外篡改 simple
`committed_controls: 4 -> 5`，gate 以 `result_taskspace_committed_controls_mismatch` 拒绝。最终 verdict 为
`pass_reacceptance`，FLA-2 恢复 `active_verified`。完整记录见对抗审查报告。
