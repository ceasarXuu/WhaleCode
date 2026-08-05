# A2 Hosted 逐项多节点绑定验证计划

- Created: 2026-08-06
- Status: V1～V3 verified-isolated / V4 pending budget / Phase A blocking
- Scope: TaskSpace Exec TX-05
- Production behavior change: 无，候选模块尚未注册

## 1. 回撤原因

先前 A2 把以下最简原型误写成了产品合同：Agent 只声明一个响应级 `hosted_node_id`，Runtime 将同一响应中的全部
Provider-hosted 事实绑定到该节点。该结论只覆盖了单响应单节点样例，不能支持 TaskSpace 的多个活跃节点：同一响应中的
不同 Hosted 动作可能由 Agent 分别归属不同节点。

先前失败合同还允许将缺声明、缺 ID 或冲突事实标记为 `unbound`，甚至以 Root owner 持久化后继续处理其他动作。这会
把不完整归属伪装成已结算状态。未绑定不是可接受的 Map 状态；它是整个 TaskSpace 响应必须拒绝的协议错误。

因此，A2 从 `passed` 回撤为 `in-progress`。既有证据仅保留以下两点：

1. Runtime 能从 Provider 原始 output item 读取 `id/item_id` 和状态；
2. Agent 不应承担 Provider 传输 ID 的复制或创造。

## 2. 必须证明的产品合同

1. Agent 在 `taskspace_exec` 中为每个带独立 Provider 身份的原始 Hosted output item 分别声明 `node_id`；绑定单位不是
   整个响应，也不是 Runtime 推断的语义任务组。
2. 同一响应的不同 Hosted 动作可以绑定不同节点；响应边界不限制 Map 归属。
3. Runtime 只做结构化的一一核对：不选节点、不按 URL、结果内容或语义相似度猜配。
4. Provider 原始 `id/item_id` 是执行事实身份，Agent 声明的 `node_id` 是归属事实；两者都不能由 Runtime 替代另一方。
5. 只有全部 Hosted 事实和声明完整、一一、唯一且节点合法时，整个计划才可进入 canonical Map admission 和 client
   dispatch。
6. 漏绑、多绑、歧义、非法节点、Provider ID 缺失或冲突时，整批拒绝：client/map 零执行、Map 零提交、Event Store
   零写入。
7. Provider 已经发生的原始输出仍保留在 provider response 和诊断日志中，但不能通过默认 Root、默认节点或
   `unbound` 降级进入 Map。Runtime 不吞结果、不重执行，也不把失败伪装成成功。Agent 显式选择 Root 节点时，仍由
   canonical Map validator 判断是否合法。
8. Provider Tool 的 completed/failed/cancelled 状态与节点生命周期正交；只要身份和归属合法，原始状态即可忠实登记。

## 3. 验证单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A2-V1 | 盘点逐项关联能力 | discovery | Responses decoder、Web/Image `ResponseItem`、历史 probe artifact | Runtime/Agent 双方可用的 Hosted 结构字段 | 确认 Provider `output_index` 是顺序权威，Agent 只需按该顺序声明，不复制 Provider ID | 得到非语义、可唯一核验的关联键，并发现当前通用 SSE decoder 丢 index 的 TX-07 接线前置 | Complexity: 一份证据矩阵；Reach/Cost: 零生产影响、零 API 费用 | 同类多项与乱序 done fixture 按 index 恢复 | TX-07 必须保留 index | verified |
| A2-V2 | 实现逐项声明候选 | API/internal | `core/src/tools/taskspace_exec/plan.rs`、`decoder.rs`、`preflight.rs` | `hosted_bindings[]` | 删除单值 `hosted_node_id`；使用有序 `{tool,node_id}` 数组并升级计划 v2 | typed plan 可以表达同响应多个 Hosted 节点 | Complexity: 重写未接生产候选 schema，不增加第二 registry；Reach/Cost: Phase A tests/snapshots 更新 | 双节点、旧字段、空字段和严格 decode 单测 | 需要修改 provider/普通 Tool schema 时回退 | verified-isolated |
| A2-V3 | 建立原子核对门禁 | internal | `provider_reconcile.rs` | complete Hosted binding set | 按 output index、数量和 Tool 类型完整核对；任一 finding 返回空 bindings | 不完整归属不能产生部分成功、默认 Root owner 或未绑定 settlement | Complexity: 扩大候选 reconciler 和 finding；Reach/Cost: 零生产接线、零 Provider 费用 | 缺/多/乱序/重复 ID/index、类型错配均整批拒绝 | Map/Store/Router 接线副作用在 TX-09/11/12/17 复验 | verified-isolated |
| A2-V4 | 验证目标模型生成能力 | provider validation | `r8_taskspace_exec_a2_probe.py`、Docker、run ledger | same-response multi-node Hosted declaration | 申请预算后运行有真实双子任务需求的最简样本；只提供标准 TaskSpace 协议，不在任务正文中喂预期绑定答案 | 避免只证明 Runtime parser，或用迎合样本伪造 DeepSeek 可用性 | Complexity: 专用探针已完成 5 项离线自检；Reach/Cost: 真实执行有明确 token、费用和耗时 | 原始 wire中至少两项 Hosted 事实分别归属两个节点；逐项核对且禁止自动扩大 repeat | 未获预算不执行；失败保持 A2 blocked | probe-ready / budget-pending |

## 4. 确定性矩阵

| Case | Provider facts | Agent declarations | Expected |
|---|---|---|---|
| A2-C01 | 0 | 0 | 接受，继续验证其他计划成员 |
| A2-C02 | 1 项 -> node A | 1 项 -> node A | 接受，真实 Provider ID 只写 node A |
| A2-C03 | 2 项 | 分别 -> node A/node B | 接受，两个 owner 独立 |
| A2-C04 | 2 项 | 均 -> node A | 接受，两项事实共享 owner |
| A2-C05 | 2 项同类 Hosted | 分别 -> node A/node B | 只有关联键能唯一核对时接受 |
| A2-C06 | 2 项 | 只声明 1 项 | 整批拒绝，零 dispatch/commit/store |
| A2-C07 | 1 项 | 声明 2 项 | 整批拒绝，零 dispatch/commit/store |
| A2-C08 | 事实与声明无法唯一对应 | 数量相同 | 整批拒绝，不按内容或语义猜配 |
| A2-C09 | 缺失或重复 Provider ID | 数量和节点合法 | 整批拒绝，不做默认 Root/unbound 降级 |
| A2-C10 | 1 项 | 节点不存在或当前不可接纳 | 整批拒绝，不选择默认节点 |
| A2-C11 | failed/cancelled 事实身份合法 | 合法节点声明 | 接受并保留原状态，不改变节点生命周期 |
| A2-C12 | outer exec 缺失或无法解码 | 1～N 项 | 响应拒绝；原始输出仅留在 response/trace，不进入 Map |

## 5. A2 完成门禁

A2 只有同时满足以下条件才能恢复为 `verified-isolated`：

1. A2-V1 找到并用源码/fixture 证明可唯一核验的逐项关联方式；已完成；
2. A2-V2/V3 的正反矩阵全部通过，且候选代码中不存在单值 `hosted_node_id`、Root fallback 或 unbound settlement；已完成；
3. A2-V4 获得单独预算并证明 DeepSeek 能在同一响应中为至少两个 Hosted 动作声明不同节点；
4. 缓存敏感面若发生变化，先通过缓存门禁并按规则申请真实缓存回归；
5. 主合同、工程计划、日志字段和测试断言对失败语义一致。

任一条件失败时，Phase B 保持阻断。不得以整响应单节点、拆请求、语义猜配、默认 Root 或“先记 unbound 后继续”作为
自动降级方案。
