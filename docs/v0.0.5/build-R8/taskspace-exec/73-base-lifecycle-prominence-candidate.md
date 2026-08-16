# Base 状态机显著性候选

- Date: 2026-08-17
- Status: implemented offline / live validation pending
- TaskSpace Base version: `3.0.4`
- Base SHA-256: `a783705f320504306fc9fca591cb1b15246b73482201a916b511f8d5cc49ec33`
- Comparison baseline: [`72-state-machine-protocol-repeat5-result.md`](72-state-machine-protocol-repeat5-result.md)

## 1. 假设

完整状态机硬合同已经位于 `taskspace_exec.description`，但上一轮五次真实运行仍有 `2/5` Waiting frontier 误选和 3 次
非法状态转换。候选检验的是载体显著性：Agent 是否需要在 Base 建立稳定的状态机工作模型，才能在选择 Tool 序列前正确使用
Tool 合同中的精确规则。

这对应 COE H-005。当前 Base 同时要求“在 meaningful work boundary 更新 lifecycle”和“不要在每个 minor Tool result 后更新”，
但没有明确 Tool outcome 与 Work completion 的关系。一次 patch 基本完成整个 `fix` 节点时，这里存在解释空间。

## 2. 单变量

TaskSpace Base 的 `TaskSpace work map` 增加一段无字段、无 JSON、无序列名的工作模型：

- Waiting 不可执行，Ready 可执行；
- 在 Ready owner 上开始 Tool work 后节点进入 InFlight；
- Tool result 只是证据，不会让 owner 成为 Completed；
- Work goal 已满足时，进入任何依赖节点前先显式完成该 Work；
- Runtime 随后根据依赖机械派生新的 Ready frontier。

保持不变：

- `taskspace_exec` Tool description、schema 和八种合法序列；
- Runtime 状态机、preflight、拒绝与反馈；
- Map projection、普通 Tool、Provider 路径与 Standard Base。

Base 负责 Agent 的工作模型；Tool description 仍是合法转换和调用形状的唯一精确操作合同。Base 不复制
`initialize_and_work`、`update_and_work`、字段名、JSON 示例或参数定义。

## 3. 离线验收

- TaskSpace Base version 从 `3.0.3` 升为 `3.0.4`，固定新 hash；
- Base profile tests `6/6` 通过；
- 新测试要求六条 lifecycle 心智模型存在，并禁止四个 Tool 序列/字段标识进入 Base；
- `cargo fmt --all -- --check` 通过。

离线验收只能证明载体与层次正确，不能证明 DeepSeek 行为改善。

## 4. 真实验证设计

建议继续使用 `single-file-fast-fix × map-request × repeat=5`，与上一轮相同：

- 主要指标：Waiting frontier 误选、same-update Waiting child transition、冗余 InFlight patch；
- 守门：5/5 业务、公开验证、隐藏 oracle 和 Map 闭合；
- 成本：requests、input/cached/uncached/output、Request 2+ 缓存命中率、Agent wall；
- 零自动重试，每轮最多 12 个 Provider requests；
- 上限：60 requests、750,000 input、20,000 output、600 秒或 1 CNY，任一达到即停。

该真实运行需要新的用户预算批准。不能复用已经结算的上一轮授权。
