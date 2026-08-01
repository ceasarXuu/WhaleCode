# R6 Phase E Finish 终结边界设计

## 1. 问题与根因

Phase D 的 live sample 暴露出初始化工具把 `Finish` 与普通 `Work` 节点复用为同一结构：两者都
必须提供 `node_id + goal`。该合同会自然诱导 Agent 把“验证、测试、总结”等尚未执行的工作写入
Finish，导致图中的唯一终点同时承担工作节点职责。

根因不是 Agent 把若干相关修复合并进同一 Work。一个连贯的软件修改可以由 Agent 在一个 Work
节点内完成，Runtime 不应强制 fork/join 或按文件、测试类型拆分。根因是工具 schema 对 Finish
暴露了本不属于终点的工作语义。

## 2. 冻结合同

`taskspace_control.initialize_map` 的节点合同改为：

| 角色 | Agent 输入 | 可承载工作 | 可获得 ordinary tool lease |
|---|---|---|---|
| Root | `node_id + goal` | 否，保存根任务语义 | 否 |
| Work | `node_id + goal` | 是，包括读取、修改、验证 | 是 |
| Finish | `node_id` | 否 | 否 |

具体规则：

1. Finish 只声明唯一终点身份，不接受 `goal`、验证计划或总结文本。
2. `finish_end` 是唯一闭合 Finish 与 Root 的动作，只接受 revision 和 Agent 原样提供的最终总结。
3. Runtime 不通过 `verify`、`test` 等关键词判断工作是否完成，也不要求存在特定名称或类型的
   验证节点。
4. Agent 自主选择 Work 粒度；验证可以是独立 Work，也可以属于一个连贯修复 Work，但不能放入
   Finish。
5. 不提供旧 `finish.goal` 的兼容解析、迁移或静默丢弃。携带该字段的输入按严格 schema 拒绝，
   且不得部分初始化 Map。

## 3. 工程切换

本次切换贯穿同一条机械链路，不增加新的架构层：

```text
tool JSON schema
  -> taskspace_control typed args
  -> handler initialize input
  -> rooted DAG initialize transaction
  -> canonical Finish node
  -> projection / snapshot
```

- 工具层为 Finish 使用独立 strict object，仅允许 `node_id`。
- handler/runtime 使用独立 Finish input，类型上不再出现 `goal`。
- 领域构造器只能创建空 goal 的 Finish；validator 拒绝 replay/snapshot 中出现非空 Finish goal 的
  非法状态，防止绕过工具入口。
- protocol 暂不新增第二套节点结构；canonical snapshot 中 Finish 的既有 `goal` 字段固定为空，
  projection 不把空值重新解释成工作目标。

## 4. 日志与错误

无兼容切换需要留下两类可审计事实：

- schema/typed args 拒绝 `finish.goal` 时保留既有 `taskspace.control_arguments_rejected` 事件；
- 领域 validator 对被篡改或损坏的非空 Finish goal 返回稳定的
  `finish_goal_not_empty`，并记录 `state_commit=false`。

日志只记录请求、拒绝码和提交事实，不加入“应先验证”等策略性建议。

## 5. 验收

确定性门禁：

```text
initialize_map schema 的 Finish properties 只有 node_id。
合法 node_id-only Finish 初始化并绑定 initial Work。
携带 finish.goal 的 JSON 在参数解析阶段失败，Map 保持空白。
直接构造或 replay 非空 Finish goal 时返回 finish_goal_not_empty。
合法 Finish snapshot/projection 不出现工作目标。
finish_end 仍要求所有 Work completed，并原样提交 final_summary。
```

Live 门禁选择 `subscription-billing-repair` 单样本运行一次，观察 Agent 是否生成
`finish: {node_id}`，以及验证工作是否只出现在 Work 执行路径。该样本只用于观察自然行为，不能据
单次结果声称成本或智能收益。

## 6. 非目标

- 不按自然语言识别“验证节点”；
- 不规定每个修复、文件或测试必须拆成独立节点；
- 不让 Runtime 自动创建、补全或重排 Work；
- 不在本主题提前完成 Phase E 的 resume/fork/crash 全矩阵；
- 不用 projection 提示 Agent 下一步应当验证什么。
