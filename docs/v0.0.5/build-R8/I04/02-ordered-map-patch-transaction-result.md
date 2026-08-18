# R8 I04 有序 Map Patch 事务修复结果

- Date: 2026-08-19
- Scope: Runtime 状态事务、TaskSpace Exec 协议与确定性测试
- Status: offline passed / cache gate pending / production validation pending

## 1. 根因

`node_patches[]` 对 Agent 表达的是有序动作，但旧 Runtime 先连续应用全部 patch，最后才统一派生一次节点 readiness。
因此 `fix -> completed` 虽然已经写入候选 Map，后续 `verify -> completed` 校验时仍会看到旧的 `waiting` 状态并被拒绝。
历史 I04 trace 中这不是 Agent 误选节点，而是 Runtime 没有兑现数组顺序语义。

## 2. 修复语义

Runtime 在候选 Map 上按数组顺序处理每个 patch，并在每一步后机械重新派生依赖状态。前序 patch 完成父节点后，后序
patch 可以操作刚解锁的子节点。候选 Map 仍只在整批全部通过后一次提交；任一步非法，当前持久化 Map 保持不变。

该行为只使用图依赖和声明顺序，不判断工作是否充分，也不替 Agent 选择节点。

## 3. 离线证据

- Rooted DAG：15/15 通过。
- TaskSpace Exec：78/78 通过。
- 正序 `fix completed -> verify completed -> finish` 成功。
- 逆序 `verify completed -> fix completed` 仍以 `TransitionInvalid` 拒绝。
- 失败候选不改变当前 Map。

## 4. 未完成验收

TaskSpace Exec 的 Agent 可见 description 已同步新规则，属于缓存敏感面。必须先通过缓存回归门禁，再申请最小真实运行预算，
确认目标模型能在一个请求中稳定提交父节点、刚解锁子节点与 Finish，并且不再产生历史额外请求。
