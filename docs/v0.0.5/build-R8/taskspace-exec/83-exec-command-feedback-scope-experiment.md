# exec_command 首轮作用域单变量实验

- Status: completed-no-promotion
- Product Authority: `../02-r8-global-constraints.md`
- Applicable Decisions: 2.4、2.8、2.9、4.1、4.2、5.4、5.5、5.7、5.8、5.9、5.13
- COE: `../../../../coe/2026-08-19-07-24-r8-taskspace-action-top-level-promotion.md`
- Budget: 用户批准总包不超过 CNY 3.00

## 目标

隔离验证模型可见的 TaskSpace client Tool 表达中，哪一项放大了顶层 `exec_command` 逃逸。每个实验臂只改一个变量，
不得改动原生 Tool identity、Base instructions、Map/DAG、状态机、拒绝逻辑、projection 或 Standard。

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
| E1 | F1 回退后，仅移除 description 中两段包含 `exec_command` 的 work 示例 | 闭集 schema、协议正文、read/finish 示例及 Runtime 均保持 B0 | 首响应逃逸明显低于 B0，初始化、业务和 Map 不回归 | 逃逸无下降或初始化/序列质量下降 |

F1 已证明与症状时序不相容：所有观测逃逸均发生在首个工作响应，早于任何成功反馈。原 F2 同样只改后续反馈，取消且不运行。
E1 与 F1 不叠加；运行前生产代码已回退到 B0。

## 工作单元

| ID | 工程动作 | 验证 | 停止边界 |
|---|---|---|---|
| E0 | 构建并固化 B0 二进制 | TaskSpace focused tests；binary attestation | 构建或离线测试失败即停 |
| E1 | B0 `repeat=5` | 顶层逃逸、完成率、请求、token、缓存、耗时、费用 | 任一基础设施/usage 门禁失败即停 |
| E2 | 实现 F1，仅改结果字段及对应 schema/tests | focused tests + cache fingerprint gate | 出现 Standard、Tool 原生合同或 wire 非预期变化即回退 |
| E3 | F1 `repeat=5` | 与 B0 同口径逐 run 对比 | F1 不通过则回退，不在其上叠加 |
| E4 | 实现并运行 E1 `repeat=5` | 与 B0 同口径逐 run 对比 | 无清晰收益即回退，不新增解释变量 |
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

## 中间结果

- B0：5/5 业务与隐藏验证通过；3/5 run、6 call 出现顶层 `exec_command`；56 requests；估算 CNY 0.17073372。
- F1：5/5 业务与隐藏验证通过；3/5 run、4 call 出现顶层 `exec_command`；48 requests；估算 CNY 0.16675288。
- B0 与 F1 的全部逃逸都与首个 `initialize_and_work` 同响应产生，发生在任何 `client_results` 反馈之前。
- 结论：F1 的 call 数波动不构成因果收益；成功反馈不是首轮逃逸的必要条件。F1 已通过 commit `efcb738f5` 完整回退。
- E1：5/5 业务与隐藏验证通过；2/5 run、6 call 出现顶层 `exec_command`；47 requests；估算 CNY 0.18867856。
- E1 的逃逸总 call 数与 B0 相同，仅集中到 2 个 run；该规模下不构成清晰方向，候选已通过 commit `2504d32e0` 回退。
- 总计：15 个有效 sample、151 requests、2,737,651 input、2,455,808 cached input、97,603 output、估算 CNY 0.52616516。
- 最终结论：成功反馈字段和两段 work 示例都不是充分根因；当前生产代码保持 B0。下一轮应研究首响应中 Provider 对未声明 Function 名的选择机制与可用的结构性约束，不继续叠加提示文字。

## Pre-Phase Plan Rebase Gate

- Rebase scope: 当前代码、H-006 根因证据和全局约束
- Material plan delta: none
- User approval: user-approved-direct: “设计实验并实施，提前批准不超过3元的预算总包”
- Gate: ready
