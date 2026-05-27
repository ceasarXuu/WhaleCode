# Action Map Growth Health E2E 设计

## 目标

这个 E2E 用来验证 TaskSpace 不是只在简单任务里产生几个事件，而是在真实用户解决问题的过程中持续驱动任务：

- 任务能被拆成多个有意义的节点。
- 节点会随着发现和推进继续生长。
- 子 agent 必须绑定到具体节点，并把结果写回节点。
- 主 agent 的普通工具调用结果要归属到当前节点。
- 已完成节点不能被继续当作工作容器复用。
- 最终状态、可观测导出、业务结果三者必须能互相印证。

## 场景

脚本：

```powershell
.\scripts\run-action-map-growth-health-e2e.ps1
```

脚本会创建一个真实临时 git repo：

```text
src/order_pipeline/parser.py
src/order_pipeline/pricing.py
src/order_pipeline/invoice.py
tests/test_parser.py
tests/test_pricing.py
tests/test_invoice.py
README.md
pyproject.toml
```

任务包含多个相关但不同的缺陷：

- parser：SKU 没有 trim/lower。
- parser：quantity 没有正数校验。
- pricing：Premium 折扣应为 10%，且大小写不敏感。
- pricing：VIP 折扣应为 15%，且大小写不敏感。
- invoice：集成路径必须按 README 先折扣、再按折后金额决定 shipping。
- test：其中一个集成测试的期望值故意和 README 冲突，要求 agent 发现并修正测试，而不是把代码改成迎合错误测试。

脚本使用真实 Whale：

```text
whale.exe exec --json --taskspace ...
```

并要求 agent：

- 创建具体 TaskSpace task/map。
- 不得只创建一个泛泛的 “fix everything” 节点。
- 维护 boundary / parser investigation / pricing investigation / implementation / validation 等节点。
- 至少 spawn 两个真实 investigation subagents。
- 子 agent 只调查和汇报，主 agent 负责最终判断和落地。
- 主 agent 完成节点后用 `taskspace_control(action=finish_node)` 关闭节点。
- 执行真实失败验证和真实通过验证。

## 健康判定

脚本同时检查业务结果和 map 生长，不允许“代码过测但 TaskSpace 没运行起来”，也不允许“map 看起来热闹但业务实际错了”。

| 指标 | 阈值 |
|---|---:|
| maps | >= 1 |
| nodes | >= 4 |
| agents | >= 2 |
| completed spawn_agent | >= 2 |
| lease_created / lease_attached / lease_released | >= 2 |
| nodes_with_results | >= 3 |
| completed_nodes | >= 2 |
| finish_node success output | >= 2 |
| later_node_created_after_completion | true |
| node title category coverage | boundary / parser / pricing / implementation / validation |
| parser investigation node has subagent result | true |
| pricing investigation node has subagent result | true |
| implementation node has main-agent tool results | true |
| validation node owns passing pytest result | true |
| post-hoc empty terminal nodes | 0 |
| real command execution | >= 4 |
| final pytest | pass |
| hidden oracle | pass |

隐藏 oracle 会直接 import 最终源码并验证 README 业务规则。它独立于 repo 内测试，避免 agent 通过修改测试或迎合错误测试得到假阳性。

## 最近真实运行

使用当前源码构建出的二进制运行：

```text
D:\whalecode-alpha\target-test\debug\whale.exe
```

最新报告：

```text
D:\whalecode-alpha\target\real-user-e2e\action-map-growth-health-order-pipeline\20260528-045550-749\artifacts\report.md
```

结果：

```text
overall: PASS
validation_exit_code: 0
hidden_oracle_exit_code: 0
nodes: 6
agents: 2
spawn_agent: 2
lease_created: 6
lease_attached: 2
lease_released: 6
nodes_with_results: 6
completed_nodes: 6
finish_node_calls: 4
later_node_created_after_completion: True
parser_investigation_used: True
pricing_investigation_used: True
implementation_node_has_main_tools: True
validation_node_has_pytest_result: True
posthoc_empty_terminal_nodes: 0
real_command_execution: 32
```

这次运行的核心结论：

- TaskSpace 生长健康：1 个 map 生长出 6 个节点，两个子 agent 分别绑定 investigation node，并由对应 subagent thread 写回结果。
- 主 agent 在 implementation node 下执行修改工具调用，在 validation node 下执行 `python -m pytest tests -q`，且该节点拥有通过结果。
- 当前源码 runtime 支持 `finish_node`，脚本按 `TaskSpace node finished:` 成功输出统计，避免旧正则误报。
- repo 内 pytest 和隐藏 oracle 都通过；隐藏 oracle 验证了 README 业务规则，避免仅靠可被 agent 修改的测试产生假阳性。

## 设计价值

这个 E2E 的重点不是每次都让模型稳定通过，而是给我们一个真实的有效性观测器：

- 如果业务和 map 都通过，说明本轮 TaskSpace 驱动有效。
- 如果业务通过但 map 失败，说明 agent 绕过或弱化了 TaskSpace。
- 如果 map 通过但业务失败，说明 TaskSpace 结构运行起来了，但 agent 的需求理解或验证策略仍不可靠。
- 如果两者都失败，说明 runtime 约束、prompt、任务拆解或模型执行路径都有问题。

因此这个脚本适合作为 TaskSpace 后续优化的核心回归场景，而不是普通冒烟测试。
