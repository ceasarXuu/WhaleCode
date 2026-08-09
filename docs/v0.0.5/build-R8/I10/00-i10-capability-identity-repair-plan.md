# I10 TaskSpace 能力身份修复计划

- Created: 2026-08-09
- Status: implementation-verified；production trace pending
- Product Authority: [`../taskspace-exec/00-product-contract.md`](../taskspace-exec/00-product-contract.md)
- Applicable decisions: Runtime 维护机械身份；Agent 不回显 `capability_id`；普通 Tool 和 Standard 保持原生
- Parent plan: [`../taskspace-exec/12-phase-b-zero-base-plan.md`](../taskspace-exec/12-phase-b-zero-base-plan.md)

## 1. 问题

当前 `TaskSpaceExecCatalog` 的同一快照已经同时生成 Agent 可见的 Exec declaration 并驱动内部 dispatch，但没有稳定身份。
Provider wire 独立记录最终 `tools_hash`，Exec 日志和性能报告无法证明它们对应同一份能力快照。工具集合变化时，成本和行为
差异因此不能可靠归因。

## 2. 修复合同

1. capability identity 由同一 effective Catalog 快照的确定性 JSON 机械计算；输入包含 Exec 外层 declaration、原样
   Hosted declarations 和驱动内部 dispatch 的 client/map capabilities。
2. 相同声明逐字得到相同 SHA-256；名称、描述、kind、schema、deferred 或 Hosted 配置变化必须改变身份。
3. Catalog、内部 dispatch、请求快照、Provider wire trace、Exec trace 和 benchmark 引用同一个值，不各自再算替代 hash。
4. `tools_hash` 继续描述 Provider 实际 wire；capability identity 描述 Runtime 发送该 wire 时使用的 TaskSpace 能力快照。
   两者并列记录，不互相冒充。
5. identity 只存在于 Runtime metadata 和日志，不进入 Agent schema、Map、Tool 参数、Provider payload 或聊天上下文。
6. Standard 的 Tool payload、wire 和日志语义不变；其 capability identity 为空。

## 3. 执行合同

- 产品权威的修改只能由用户确认；工程证据只能调整本计划。
- 不建立 identity registry、数据库、版本协商、兼容 reader 或第二套 Tool catalog。
- 不把动态 Map、node、request、Provider output 或 session 状态纳入 identity。
- 任一实现需要修改普通 Tool schema、让 Agent 填写身份或改变 Provider payload 时立即停止。

## 4. 工作单元

| ID | 目标 | 修改位置 | 动作 | 收益 | 副作用 | 验证 | 状态 |
|---|---|---|---|---|---|---|---|
| ID-01A | 建立唯一身份 | `taskspace_exec/catalog.rs`、Router | 从最终声明序列计算并保存 identity，由 TaskSpace Router 暴露同一值 | declaration 与 dispatch 可证明来自同一快照 | 增加一个只读字符串字段；无新状态机 | 顺序确定性、语义变更、Hosted 变更、Agent schema 无字段测试 | verified |
| ID-01B | 沿请求链传播 | Prompt metadata、response scope、Exec tracing、provider wire tracing | 从 Router 机械传递 identity，并在 handler 使用前核对 | wire 与执行可逐 request 对账 | Provider trace schema 增加可选字段；Standard 为 null | HTTP/WS、成功/拒绝、Standard fixtures | verified |
| ID-01C | 接入报告消费 | TaskSpace Exec observer 与 fixture | 汇总唯一 identity；缺失、多个值或与 wire 事件不一致时判不可比较 | I07/I08 不再把能力变化误判为任务变化 | 旧 artifact 无该字段时只作为历史，不伪造身份 | 正常、缺失、冲突 fixture | verified |
| ID-01D | 离线收口 | 定向测试、zero-base、cache gate、文档 | 固化证据并重映射 I10 | 为 VA-02/VA-03 提供可信前置 | 不产生 Provider 成本 | 全部 PASS；缓存指纹变化需按门禁处理 | verified |

## 5. 产品决策增量

| Phase | Decision Surface | Implemented / Observed Semantics | Authority Coverage | Classification | Required Action |
|---|---|---|---|---|---|
| ID-01 | Agent/Runtime/Tool 边界 | 只增加 Runtime 内部机械身份，不改变 Agent 动作、Map 生命周期和 Tool 语义 | 产品合同现有边界 | engineering-only | 完成后复核无 Agent-visible 字段 |

## 6. 后续顺序

`ID-01A -> ID-01B -> ID-01C -> ID-01D -> VA-02 -> VA-03 -> VA-04B`。真实 Provider 验证仍需独立预算。

## 7. 离线结果

- Runtime 身份链提交：`8481d24bc`；观察器提交：`d669c62a0`。
- TaskSpace Exec `58/58`、Core `1857 passed / 3 ignored`、workspace check、zero-base、性能观察 fixture 全部通过。
- `ResponsesApiRequest` 等值测试证明 identity 只进入 Runtime metadata 与 trace，不改变实际 provider request。
- 缓存门禁发现 accepted snapshot 的 Standard Skills developer 内容相对当前环境已漂移，并将提交标记为可比较候选、
  发布继续阻断；该差异不在 TaskSpace identity 或 Tool payload。是否晋升缓存基线仍需用户批准的专用真实回归。
- I10 的静态缺口已修复，但在 VA-02/VA-03 取得当前生产 trace 前保持 `verifying`，不提前关闭。
