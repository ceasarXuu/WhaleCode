# R7 五层架构 FLA-7 验收结果

- 日期：2026-07-23
- 状态：完成
- 范围：生命周期恢复、三种 projection policy 的共享事实与载体一致性

## 结论

FLA-7 没有修改 TaskSpace 的语义或 Agent 行动规则。现有生产路径已经共享同一份 canonical Map、renderer、
状态机与结果链；本阶段补齐了之前缺失的可重复验证门禁。

LC-01 至 LC-05 明确降为 FLA-3.5 前的历史证据，不再作为当前合同执行。当前 FLA-7 gate 只覆盖 LC-06 至
LC-12：append retry、同 revision 新请求、request read 后 mutation、always replacement、resume、fork 和
compaction。

## 工程落地

1. 冻结 revision 4/5 两份完整 Map，并独立重算 canonical SHA256 和 event-chain head。
2. 用生产 renderer 生成三种 policy 的精确载体 golden；测试代码不复制 projection 构造逻辑。
3. `freeze-r7-five-layer-fixtures.ps1` 默认只比较，只有显式 `-Update` 才更新 golden，避免语义漂移被静默接受。
4. 五层总 gate 新增 `FLA-7`，要求 authority、production manifest、oracle、golden 和生产 renderer 同时一致。

## 验证

- FLA-7 fixture freezer：通过。
- 生产 renderer fixture：2 个 Map revision、3 种 policy，全部通过。
- canonical Map hash：revision 4/5 全部匹配。
- event-chain head：revision 4/5 全部匹配。
- FLA-6 三个可选实验保持禁用，未混入生产路径。

FLA-8 将只比较五层改造后的四个运行臂：Standard、map-always、map-append、map-request。首轮每个样本每臂
重复 3 次，完成后暂停，不自动扩大到 10 次。
