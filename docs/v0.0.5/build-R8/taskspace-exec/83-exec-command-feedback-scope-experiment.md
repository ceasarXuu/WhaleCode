# exec_command 反馈作用域单变量实验

- Status: in-progress
- Product Authority: `../02-r8-global-constraints.md`
- Applicable Decisions: 2.4、2.8、2.9、4.1、4.2、5.4、5.5、5.7、5.8、5.9、5.13
- COE: `../../../../coe/2026-08-19-07-24-r8-taskspace-action-top-level-promotion.md`
- Budget: 用户批准总包不超过 CNY 3.00

## 目标

隔离验证成功反馈中的裸 `tool="exec_command"` 是否放大下一轮顶层逃逸。实验不得改动请求侧 client Tool catalog、原生
Tool identity、canonical 示例、Base instructions、Map/DAG、状态机、拒绝逻辑、projection 或 Standard。

## 执行合同

- 全局约束是产品权威；工程证据只能调整本实验计划，不能静默改写产品约束。
- 每次只改变一个反馈表达变量。候选不通过时完整回退，再开始下一候选。
- 所有真实运行使用 Docker、`deepseek-v4-flash`、`subscription-billing-repair`、`map-request`。
- 每轮真实运行前已有账本记录；零自动重试。基础设施失败、usage 缺失、新的协议异常或预算越界立即停止。

## 假设与候选

| ID | 唯一变量 | 保持不变 | 支持信号 | 否定信号 |
|---|---|---|---|---|
| B0 | 当前反馈 `client_results[].tool` | 当前生产代码 | 建立当前版本逃逸率 | N/A |
| F1 | 仅把 Agent-visible 结果字段改为 `executed_client_tool`，值和结果不变 | 请求 schema、Tool 名、参数、执行与其余反馈 | 逃逸明显低于 B0，业务和 Map 不回归 | 逃逸无下降或出现新理解错误 |
| F2 | 仅在 F1 不通过并回退后启用；结果用稳定 `call_index` 关联原请求，不重复 Tool 名 | 其余全部保持 B0 | 逃逸下降且并行结果仍可准确关联 | 结果关联不清或行为回归 |

F2 是条件候选，不与 F1 叠加。F1 若获得清晰收益则不执行 F2。

## 工作单元

| ID | 工程动作 | 验证 | 停止边界 |
|---|---|---|---|
| E0 | 构建并固化 B0 二进制 | TaskSpace focused tests；binary attestation | 构建或离线测试失败即停 |
| E1 | B0 `repeat=5` | 顶层逃逸、完成率、请求、token、缓存、耗时、费用 | 任一基础设施/usage 门禁失败即停 |
| E2 | 实现 F1，仅改结果字段及对应 schema/tests | focused tests + cache fingerprint gate | 出现 Standard、Tool 原生合同或 wire 非预期变化即回退 |
| E3 | F1 `repeat=5` | 与 B0 同口径逐 run 对比 | F1 不通过则回退，不在其上叠加 |
| E4 | 条件实现和运行 F2 `repeat=5` | 与 B0 同口径；仅 F1 未通过时执行 | 预算不足或产生关联歧义即停 |
| E5 | 若缓存门禁确认 provider prefix 敏感，执行专用最简缓存回归 | 专用 runner 结果与当前指纹一致 | 最多 CNY 0.50；失败不晋升 |

## 验收

- 主指标：TaskSpace 顶层 `exec_command` 逃逸 run 数和 call 数。
- 正确性：业务、公开验证、隐藏 oracle、Map 闭合均不得低于 B0。
- 行为回归：不得新增 schema/JSON、Waiting、TransitionInvalid、单独空初始化或其他顶层 client Tool 异常。
- 成本：报告 request、input、cached/uncached input、output、wall time 和估算费用；成本不是本实验的首要晋升条件。
- 证据限制：`repeat=5` 只用于方向判断；即使 0/5，也不宣称理论上彻底消除。

## 预算

- 行为实验最大 15 个 sample run、约 190 个 Provider request、3,000,000 input、100,000 output、CNY 2.50、90 分钟。
- 缓存专用回归预留 CNY 0.50；总包不超过 CNY 3.00。
- 账本：`WAR-20260819-084538-R8-EXEC-FEEDBACK-SCOPE-AB`。

## Pre-Phase Plan Rebase Gate

- Rebase scope: 当前代码、H-006 根因证据和全局约束
- Material plan delta: none
- User approval: user-approved-direct: “设计实验并实施，提前批准不超过3元的预算总包”
- Gate: ready

