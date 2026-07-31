# R8-I09 Store Hydrate 完整性调查计划

- Created: 2026-07-31
- Issue: R8-I09
- Status: planned
- Change policy: 调查阶段不修改产品行为，不运行真实 Whale Agent

## 1. 调查问题

Map 已经被定义为独立持久化的 canonical 事实。任何 Session、resume、fork、child 或进程重启都应按
`map_id` 从同一 Store 读取，并在进入 Runtime 前满足与写入路径相同的 schema 和 rooted-DAG 不变量。

R8-I09 不预设历史文档中的 hydrate 缺口仍然存在，先回答：

1. 当前所有 Store -> Runtime 入口有哪些，分别由 resume、fork、child 和普通 Session 如何触发？
2. 写入路径使用的 canonical schema、role、revision 和 rooted-DAG validator 分别在哪里？
3. hydrate 是否复用完全相同的不变量，还是只检查 `map_id`、反序列化成功等局部条件？
4. hydrate 失败前后，Runtime cache、active handle 和 Store transaction 是否保持原子不变？
5. 旧 schema、孤立节点、多起点、多终点、Root/Finish 不可达和错误 role 分别如何处理？
6. 合法多父 DAG、非零 revision 和已关闭后 reopen 的 Map 能否正常恢复？
7. Map Store 缺失或损坏时，是否仍存在 rollout、Session snapshot 或其他静默重建路径？
8. Standard 是否完全不经过该路径？

## 2. 工作单元

| ID | 目标 | 位置 | 动作 | 产出 | 验证 | 安全停止 |
|---|---|---|---|---|---|---|
| I09-A | 盘点全部读取入口 | Map Store、Session resume、fork、child attach、Runtime cache | 从每个入口追踪到 canonical Map 安装点 | 带源码锚点的入口图 | 每个生产入口有调用者和安装点 | 仅文档，无行为变更 |
| I09-B | 对比读写不变量 | initialize/mutation validator 与 hydrate path | 建立 schema、DAG、role、revision、terminal 校验矩阵 | invariant parity matrix | 每个不变量有实现锚点或明确缺口 | 不先抽象共享 validator |
| I09-C | 证明失败原子性 | Store record、Runtime cache、active handle | 用现有或新增 fixture 注入非法记录并观察前后状态 | 原子失败证据 | 非法输入失败且旧状态不变 | 不做自动修复和 fallback |
| I09-D | 证明合法恢复 | 多父 DAG、非零 revision、closed/reopen、fork/child | 执行确定性恢复测试 | 合法恢复证据 | 所有入口读取同一 map_id/revision | 不运行真实 Agent |
| I09-E | 排查第二事实源 | rollout restore、Session snapshot、兼容与 fallback 路径 | 搜索并逐调用点证明是否可重建 canonical Map | 删除候选或无残留结论 | Store 缺失时确定性失败 | 调查完成前不删除代码 |
| I09-F | 根因和方案审查 | 只基于 A-E 事实 | 判断是局部漏校验、validator 分叉还是 Store ownership 错误 | 根因、方案与影响面报告 | 用户确认重大结构调整后再实施 | 本单元不改产品行为 |

## 3. 必须覆盖的场景

| 场景 | 必须核对 |
|---|---|
| 合法 active Map | 相同 map_id、revision、nodes、edges 和 terminal 事实 |
| 合法多父 DAG | 多入边不被误判为树结构错误 |
| 合法 closed/reopen 历史 | terminal history 和旧 Work 事实不倒退 |
| schema version 错误 | 安装前失败，不进入 cache |
| Root/Finish role 错误 | 确定性失败，不自动改角色 |
| 孤立或不可达节点 | rooted-DAG 校验失败 |
| 多起点或多终点 | 唯一 Root/Finish 校验失败 |
| Store 缺失或损坏 | 显式失败，不从 rollout 重建 |
| 已存在 active cache 时 hydrate 失败 | 原 cache、handle 和 Store 状态均不变 |
| resume/fork/child | 读取同一 canonical Map，不复制或重建 |

## 4. 调查验收

R8-I09 进入设计审查前必须满足：

- 所有生产 hydrate 入口和 canonical 安装点已穷举；
- 读写不变量逐项对比，没有“等价校验”等未经证明表述；
- 至少一个合法复杂 Map 和每类非法 Map 有确定性证据；
- hydrate 失败的 cache、handle 和 Store 原子性已证明；
- rollout、Session 和兼容路径是否能重建 Map 有明确源码结论；
- 根因能够定位到具体责任边界，而不是笼统归因于 Store；
- 不新增自动修图、fallback、双 validator 或第二事实源。

调查完成后先向用户汇报并讨论方案，不自动进入实现。
