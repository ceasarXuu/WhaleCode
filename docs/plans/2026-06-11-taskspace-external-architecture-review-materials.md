# TaskSpace 0.0.3 外部架构审查材料清单

日期：2026-06-11

关联文档：
- [TaskSpace 0.0.3 架构与问题总结](./2026-06-11-taskspace-0.0.3-architecture-and-issues.md)
- [TaskSpace 认知状态工程化方案](./2026-06-04-taskspace-cognitive-state-engineering-plan.md)
- [TaskSpace Runtime 重构实施方案](./2026-05-27-taskspace-runtime-rearchitecture-implementation-plan.md)

## 背景

当前 TaskSpace 0.0.3 已经具备足够材料支持战略判断和架构方向讨论：
- TaskSpace runtime 已经接入真实 Whale 执行路径。
- task/map/node/lease/result/viewer/benchmark harness 等核心骨架已经跑通。
- E3 真实样本暴露出明确负向信号：TaskSpace 当前能运行，但尚未证明 utility 正收益。
- 当前主要矛盾不是“能不能跑”，而是“TaskSpace 的结构化工作方式是否真正改善 agent 的问题状态管理”。

因此，外部审查可以先围绕以下问题进行：
- TaskSpace 作为 task-level state manager 的方向是否成立。
- 当前 runtime / prompt / graph / subagent 协作边界是否合理。
- 0.0.3 暴露的问题更像是机制设计缺陷、实现缺陷、测试噪声，还是 benchmark 选择问题。
- 0.0.4 应优先修复行为模式，还是优先补齐评估和可观测性。

但如果要进一步做精确根因分析、0.0.4 PRD、issue 级设计或代码改造方案，最好补充下面这些材料。

## 材料优先级

| 优先级 | 需要的信息 | 为什么重要 | 预期用途 |
|---|---|---|---|
| P0 | 每个 E3 pair 的 TaskSpace trace / graph dump | 判断 node 膨胀到底发生在规划、调查、patch 还是 validation | 定位 TaskSpace 行为劣化阶段，避免只看最终 pass/fail |
| P0 | `recover-accuracy-log` 两个 TaskSpace timeout pair 的完整日志 | 这是最清晰负向信号，能定位 TaskSpace 为什么把 5/5 任务变成 3/5 | 精确分析 timeout 是调度、验证、patch、环境还是模型行为导致 |
| P0 | Standard vs TaskSpace 的 changed files / diff / validator stdout-stderr | 判断失败是 patch 错、验证慢、环境噪声，还是 TaskSpace 行为偏离 | 建立 paired 对照证据，避免把 benchmark 噪声误判为架构问题 |
| P0 | 当前 `taskspace_control` tool schema 和注入 prompt | 判断行为问题是 schema 不足、prompt 不足，还是 runtime gate 不足 | 决定 0.0.4 应改工具契约、提示词还是 gate |
| P1 | subagent spawn prompt、返回 result、主 agent 如何采信 | 判断 subagent 是贡献证据还是制造噪声 | 分析 subagent 是否真正降低主 agent 上下文压力和幻觉 |
| P1 | 每个样本的 timeout budget、validator 平均耗时、失败分类 | 区分 agent 失败和 benchmark/validator 失败 | 清洗 E3 证据，提升结论可信度 |
| P1 | viewer snapshot 示例 | 判断当前状态对人类和 agent 是否真的可恢复 | 验证可观测性是否能支撑审查、调试和后续恢复能力 |
| P2 | TaskSpace 默认启用策略的产品判断 | 决定是否需要 thin / standard / deep 模式 | 明确 TaskSpace 是默认模式、复杂任务模式，还是渐进增强能力 |
| P2 | 0.0.4 预计投入窗口和能改动的模块边界 | 决定方案应该偏 runtime、prompt、harness 还是 benchmark | 控制设计范围，避免外部审查给出无法落地的大方案 |

## P0 材料说明

### 1. E3 Pair Trace / Graph Dump

每个 E3 pair 至少需要保存 TaskSpace 侧的完整结构化轨迹：
- task 创建和路由记录。
- map 初始状态。
- node 创建、状态切换、边关系变化。
- node lease 绑定到主 agent 或 subagent 的记录。
- node result 写回记录。
- validator 调用前后的状态。
- timeout / failure 时最后一个稳定 graph snapshot。

核心审查问题：
- node 是在一开始就过度拆分，还是执行过程中被不断追加。
- graph 是否体现真实依赖关系，还是只是线性 todo list 的另一种写法。
- node 生命周期是否清晰，还是大量节点停留在 running / pending / blocked。
- 主 agent 是否基于 node result 更新 map，还是只是把 TaskSpace 当成记录器。

### 2. `recover-accuracy-log` Timeout 完整日志

这两个 pair 是 0.0.3 最值得优先分析的负向证据，因为它们呈现出明确对照：
- Standard 侧可以完成。
- TaskSpace 侧发生 timeout。
- 任务本身不是纯环境失败。

需要材料：
- 用户原始 prompt。
- Standard 完整 transcript。
- TaskSpace 完整 transcript。
- TaskSpace trace event。
- graph dump。
- validator stdout / stderr。
- changed files。
- timeout 前最后 5 分钟内的工具调用、subagent 返回、node 状态变化。

核心审查问题：
- TaskSpace 是否把高内聚任务拆成了过细节点。
- 主 agent 是否在调度上消耗过多 token 和 walltime。
- subagent 是否贡献了可用证据，还是放大了重复阅读。
- validation 是否被重复调用或卡在无效路径上。
- timeout 前是否已经存在足够信息但没有被主 agent 收敛使用。

### 3. Standard vs TaskSpace Diff / Validator Evidence

每个 pair 应该并列记录：
- Standard changed files。
- TaskSpace changed files。
- Standard patch diff。
- TaskSpace patch diff。
- validator command。
- validator stdout。
- validator stderr。
- exit code。
- walltime。
- 是否触发 cleanup。

核心审查问题：
- TaskSpace 失败是因为没有改对代码，还是改对了但验证路径失败。
- TaskSpace 是否产生额外无关改动。
- TaskSpace 是否因为更复杂的路径导致更高 timeout 风险。
- Standard 成功是否只是碰巧、过拟合或 validator 宽松。

### 4. `taskspace_control` Schema 与 Prompt 注入

需要完整导出：
- `taskspace_control` 所有 action。
- 每个 action 的必填字段、可选字段、错误返回。
- runtime gate 的拦截规则。
- taskspace mode 注入给主 agent 的系统提示词。
- node 绑定、result 写回、subagent 使用相关 prompt。

核心审查问题：
- tool schema 是否鼓励“记录行为”，但没有鼓励“决策行为”。
- prompt 是否真的把主 agent 推到问题状态管理者角色。
- runtime gate 是否只保证形式合规，而不能保证图有用。
- agent 是否知道什么时候应该新建 task、更新 map、复用 result、质疑 result。

## P1 材料说明

### 1. Subagent Spawn 与 Result 采信链路

需要导出：
- subagent 类型：default / explorer / worker。
- spawn 时传入的 prompt。
- subagent 可见上下文。
- subagent 返回的原始 result。
- result 被写入哪个 node。
- 主 agent 后续是否读取或引用该 result。
- 主 agent 对该 result 是否做过有效性标记或复核。

核心审查问题：
- subagent 是否有清晰任务边界。
- subagent 是否避免重复阅读。
- result 是否足够支持主 agent 决策。
- 主 agent 是谨慎采信，还是无条件相信。
- 主 agent 是否忽略了 result 导致 TaskSpace 只增加成本。

### 2. Timeout Budget 与失败分类

需要对每个样本记录：
- 总 timeout budget。
- agent 执行 walltime。
- validator 平均 walltime。
- validator 单次最大 walltime。
- 工具调用数量。
- subagent 数量。
- node 数量。
- edge 数量。
- 失败分类。

建议失败分类：
- `agent_patch_wrong`：代码修改方向错误。
- `agent_no_patch`：未产生有效 patch。
- `agent_validation_loop`：验证失败后无法收敛。
- `taskspace_overhead_timeout`：调度和图维护成本导致 timeout。
- `validator_slow_or_flaky`：validator 本身慢或不稳定。
- `environment_noise`：Docker、依赖、文件系统、网络等外部噪声。
- `audit_unclean`：结果看似成功但证据隔离不足，不能纳入 clean utility。

### 3. Viewer Snapshot 示例

需要至少提供：
- 小任务 snapshot。
- 中等任务 snapshot。
- 失败任务 snapshot。
- timeout 前 snapshot。
- subagent 多节点并发 snapshot。

核心审查问题：
- viewer 是否让人类快速看懂任务状态。
- graph 是否体现因果和依赖，而不是列表。
- result 是否容易追溯。
- 失败时是否能看出卡点。
- 未来是否能支持恢复、复盘、审查。

## P2 材料说明

### 1. 默认启用策略

当前需要明确产品判断：
- TaskSpace 是否未来默认开启。
- 是否允许用户手动进入但不退出。
- 是否需要 thin / standard / deep 三档。
- 简单任务是否也承担 TaskSpace 初始化成本。
- 是否接受“低复杂度不明显收益，中高复杂度追求净收益”的产品定位。

这个判断会影响 0.0.4 的设计重心：
- 如果默认开启，必须优化启动成本和低复杂度不拖累。
- 如果只用于复杂任务，必须设计复杂度识别或用户入口。
- 如果分档运行，必须定义每档的 runtime gate、prompt、tool exposure 和 graph detail。

### 2. 0.0.4 投入窗口与模块边界

外部审查需要知道 0.0.4 能改到什么程度：
- 是否允许改 `taskspace_control` schema。
- 是否允许改 runtime gate。
- 是否允许改 prompt injection。
- 是否允许改 subagent spawn 协议。
- 是否允许改 graph data model。
- 是否允许改 benchmark harness。
- 是否允许改 viewer。

如果改动窗口很小，方案应偏 prompt / harness / logging。
如果改动窗口中等，方案可以改 schema / gate / subagent result contract。
如果改动窗口较大，方案可以重构 TaskSpace 为更明确的问题状态管理 runtime。

## 外部审查建议问题

建议把以下问题直接交给外部 reviewer：

1. 0.0.3 的负收益更像是 TaskSpace 架构方向错误，还是当前执行协议不足？
2. 当前 node 粒度是否过细？一个合理 node 应该对应什么级别的高内聚工作单元？
3. 主 agent 应该如何被约束为“问题状态与模型管理者”，而不是线性执行者？
4. `taskspace_control` 的 schema 是否足够表达问题状态、证据、假设、决策和风险？
5. subagent result 是否应该更结构化，还是保持弱约束以适配开放任务？
6. graph 对 agent 的价值来自规划、记忆、并行、可观测性，还是失败恢复？0.0.4 应优先证明哪一个？
7. E3 benchmark 当前是否足以评估 TaskSpace，还是需要更适合中高复杂度工程任务的样本？
8. 默认启用 TaskSpace 是否合理？如果合理，最低可接受的低复杂度 overhead 是多少？

## 当前可先给出的结论

在没有补齐上述 P0/P1 材料前，可以给出战略级结论，但不宜给出 issue 级修复结论。

可以判断：
- TaskSpace 0.0.3 的工程路径已经成立。
- 0.0.3 没有证明 utility 正收益。
- 当前关键问题集中在行为协议和问题状态管理，而不是单纯 viewer 或 benchmark 外围。
- 0.0.4 不应继续堆更多表层功能，而应聚焦让主 agent 真正围绕 task map 管理问题状态。

暂不应武断判断：
- TaskSpace 架构方向已经失败。
- subagent 天然制造噪声。
- graph 驱动天然比线性模式差。
- timeout 全部来自 TaskSpace overhead。
- 只靠 prompt 就能修好。

## 最小补充包建议

如果只能准备一份最小审查包，建议包含：

```text
1. TaskSpace 0.0.3 架构与问题总结
2. recover-accuracy-log 两个 timeout pair 的完整对照材料
3. taskspace_control schema + prompt injection
4. 每个 E3 pair 的 graph dump 汇总
5. Standard vs TaskSpace diff + validator stdout/stderr
```

这五类材料足够让外部 reviewer 从战略讨论推进到根因定位。

