# Natural User TaskSpace E2E 设计

## 目标

这套 E2E 验证的是 TaskSpace 机制有效性，而不是只验证程序不会报错。

它和 `run-action-map-growth-health-e2e.ps1` 的区别是：

- 用户 prompt 不出现 `TaskSpace`、`map`、`node`、`subagent`、`taskspace_control` 等内部概念。
- 用户只像真实接手项目一样提出问题：读代码、按 README 修正业务、跑测试、解释改动。
- 测试从 runtime rollout 和 observability 反向检查 agent 是否主动建立 task/map/node，并把实现和验证归属到合适节点。

## 脚本

```powershell
.\scripts\run-action-map-real-user-e2e.ps1
```

脚本仍然用 `whale exec --json --taskspace` 开启 TaskSpace 模式。区别在于用户输入是自然需求，不明示任何 TaskSpace 操作。

## 自然用户输入

真实 prompt 保存在每次运行的 `artifacts/user-prompt.txt`，内容大意是：

```text
I just inherited this small order-pipeline project. The amount calculation and tests look inconsistent.
Please handle it the way you would in a real project handoff: read the code and README first, find the root cause, and fix it.

The README is the source of product truth. If a test expectation conflicts with the README, update the test to match the README.
Please run the necessary tests at the end and briefly explain what you changed and why.
```

脚本会检查 prompt 不包含以下内部词：

```text
taskspace|action map|\bmap\b|\bnode\b|subagent|spawn_agent|taskspace_control
```

## 场景复杂度

临时 repo 是一个订单流水项目，包含：

- parser 缺陷：SKU 未 trim/lowercase。
- parser 缺陷：quantity 未校验正数。
- pricing 缺陷：Premium 折扣是固定减 10，而不是 10%。
- pricing 缺陷：Premium/VIP 大小写敏感。
- invoice 测试冲突：测试期望 `45.0`，但 README 规则要求折后低于 50 时加运费，所以正确值是 `50.0`。

这要求 agent 读 README、读代码、读测试、识别测试和产品规则冲突、修改代码和测试、跑验证。

## 通过条件

测试同时要求业务正确和 TaskSpace 结构健康：

| 指标 | 阈值 |
|---|---:|
| prompt_leaks_internal_concepts | false |
| maps | >= 1 |
| nodes | >= 4 |
| nodes_with_results | >= 3 |
| completed_nodes | >= 2 |
| finish_node_calls | >= 1 |
| node title coverage | boundary/parser/pricing/implementation/validation |
| implementation_node_has_main_tools | true |
| agent_ran_passing_pytest | true |
| pytest_owned_by_validation_node | true |
| ordinary_tool_before_taskspace_binding | false |
| posthoc_empty_terminal_nodes | 0 |
| final pytest | pass |
| hidden oracle | pass |

`agent_ran_passing_pytest` 只证明 agent 在真实执行流里跑过 pytest 且通过，不等于 TaskSpace 机制有效。

`pytest_owned_by_validation_node` 才是机制归属检查：用 `call_id` 把 validation/regression node 的 result body 和 rollout 中对应的 function_call arguments 关联起来，要求对应命令包含 `pytest`，且 result body 有通过标记。

`ordinary_tool_before_taskspace_binding` 检查 TaskSpace runtime 约束是否生效：在第一个 `taskspace_control(start_task|route_task|create_node)` 绑定 task/node 之前，不允许先出现 `shell_command`、`apply_patch`、`spawn_agent` 这类普通工作调用。

## 最近运行结果

最新运行：

```text
D:\whalecode-alpha\target\real-user-e2e\action-map-natural-user-order-pipeline\20260528-150009-462\artifacts\report.md
```

结果：

```text
overall: FAIL
prompt_leaks_internal_concepts: False
validation_exit_code: 0
hidden_oracle_exit_code: 0
maps: 1
nodes: 2
completed_nodes: 1
nodes_with_results: 2
finish_node_calls: 1
has_boundary_node: True
has_parser_node: False
has_pricing_node: False
has_implementation_node: False
has_validation_node: True
agent_ran_passing_pytest: True
pytest_owned_by_validation_node: True
ordinary_tool_before_taskspace_binding: False
first_taskspace_binding_evidence: lease_created
pytest_owner_node_title: Run tests and verify
```

## 结论

这次失败是有效发现，不是脚本阈值误报。

agent 在自然用户请求下完成了业务修复，`pytest` 和隐藏 oracle 都通过；而且 runtime 确实先创建并绑定了 task/node，再开始普通工具调用。最新运行形成了 2 个节点：

```text
node-1 Read project context
node-2 Run tests and verify
```

但它仍没有自然拆出 parser/pricing/implementation 等业务语义子节点；node-1 在被后续节点接管时回到 `ready`，只有最终验证节点是 `completed`。这说明当前机制已经能证明“先绑定 task/node 后工作”和“验证结果可以归属到验证节点”，但还不能证明“用户不知道 TaskSpace 概念时，agent 会持续维护健康、可收敛、语义清晰的任务结构”。

这套 E2E 应作为后续 TaskSpace runtime/prompt 优化的核心有效性门槛。
