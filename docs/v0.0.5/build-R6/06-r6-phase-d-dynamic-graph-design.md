# R6 Phase D 动态图与执行因果设计

> 状态：已完成
> 范围：Rooted DAG 的动态变更、活动前沿、显式 rework 与观测  
> 非目标：Runtime 评价任务语义、自动补图、自动合并 Agent 意图

## 1. 设计目标

Phase D 让同一张 Rooted DAG 同时承担依赖图和状态机职责。Agent 可以显式创建 fork、join、
diamond、block、unblock 和 rework；Runtime 只执行不可绕过的机械校验、原子提交和忠实反馈。

必须保持：

1. `TaskRoot` 是唯一源点，`Finish` 是唯一汇点，所有节点位于 Root 到 Finish 的路径上。
2. 入边表示节点启动前必须成立的执行前置事实；多入边采用全部满足。
3. Runtime 不推断任务含义，不替 Agent 选择节点、边、rework 路径或冲突解决方案。
4. 失败事务不改变 revision、节点、边、lease、result 或 event journal。

## 2. 图变更硬规则

`mutate_graph` 是带 `expected_revision` 的批量事务。完整候选图通过以下校验后一次提交：

- 节点、边、唯一 Root/Finish、无环、度数和可达性不变量；
- 已存在边才能删除，已存在节点 ID 不能再次添加；
- `Running`、`Blocked`、`Completed` Work 节点的入边不可新增或删除；
- 可修改 `Pending`、`Ready` Work 或未闭合 Finish 的入边；
- 可修改已执行节点指向未来节点的出边，因为它没有改写该节点启动时的前置事实；
- stale revision 直接返回机械冲突，不自动重放、不合并、不重试 Agent 意图。

被拒绝的事务统一满足 `state_commit=false` 和 `partial_commit=0`。反馈只包含稳定 violation code、
当前 revision 和涉及对象。

## 3. Readiness

Readiness 是图和节点状态的机械派生，不是独立权威：

- Work/Finish 的所有直接前驱均满足时，`Pending -> Ready`；
- Ready 节点因图变更或 rework 出现未满足前驱时，`Ready -> Pending`；
- Root 的 `Open` 视为满足，Work 只有 `Completed` 视为满足；
- Running、Blocked、Completed 不由 readiness 自动改写。

因此向 Ready 节点添加未完成前驱不会留下错误活动前沿，移除前驱也不会绕过全满足规则。

## 4. 显式 Rework

`rework` 是 Agent 对 `Completed` Work 发出的显式状态变更：

```text
Completed --rework--> Ready
```

它不删除历史 result 或 event。Runtime 只做因果校验：若任一传递后继 Work 已经是
`Running`、`Blocked` 或 `Completed`，则拒绝 rework，因为旧完成事实已经被下游执行消费；
若后继均未开始，则允许 rework，并由 readiness 机械地把不再满足的 Ready 后继回退到 Pending。

Runtime 不自动创建替代节点，不判断 rework 是否合理，也不生成修复建议。

## 5. 活动前沿与 Lease

活动前沿只包含可承载普通工具工作的 Work：

- `ready_work_node_ids`：可由 Agent bind 的 Work；
- `running_work_node_ids`：已持有合法 lease 的 Work；
- `current_node_id`：Main Agent 当前绑定，必须属于 running Work；
- `finish_ready`：单独暴露，不能混入普通工作前沿。

TaskRoot 和 Finish 永不承载 ordinary tool lease。Viewer 直接显示真实边、入度/出度、活动前沿和
当前 lease，不构造父子层级。

## 6. 观测与验收

每次事务至少记录 operation、revision、`state_commit`、violation code；基准报告增加：

- node/edge/revision；
- max depth、max indegree、max outdegree；
- ready/running/blocked frontier；
- root count、finish count、unreachable count、cycle count；
- graph mutation/rework/stale reject 的动作路径。

测试矩阵：静态 fork/join、diamond、Ready 双向回退、运行中入边改写拒绝、rework 成功与因果拒绝、
同 revision 双写、混合 add/remove 原子失败、Root/Finish lease 非法状态。

三臂验证保持 Standard、冻结 R5、当前 R6。R5 使用冻结证据，不在 R6 分支伪装重跑旧实现。
branch-join 和 rework 各运行一次；若 Agent 自然形成 chain，只记录能力现象，不由 Runtime 补图。

## 7. 外部依据

1. [Apache Airflow DAG](https://airflow.apache.org/docs/apache-airflow/stable/core-concepts/dags.html)：
   DAG 以显式 upstream/downstream 表达依赖，默认等待全部上游成功。
2. [Kubernetes API Concepts](https://kubernetes.io/docs/reference/using-api/api-concepts)：
   `resourceVersion` 用于检测丢失更新，stale 写入由服务端拒绝，冲突解决留给客户端。
3. [petgraph acyclic](https://docs.rs/petgraph/latest/petgraph/acyclic/index.html)：
   DAG 包装器以不变量方式保证无环，支持在变更边界进行结构校验。

这些资料只用于验证通用机械模型，不引入调度器的语义决策能力。
