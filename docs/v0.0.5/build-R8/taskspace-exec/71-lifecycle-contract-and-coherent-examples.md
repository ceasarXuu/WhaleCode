# lifecycle 合同与连续示例修复

- Date: 2026-08-17
- Scope: `taskspace_exec` Agent-visible Tool description
- Live Whale run: 未执行

## 1. 问题

节点依赖、状态派生、Tool 启动和完成规则此前分散在顶层说明、sequence branch、字段说明和设计文档中，没有一个完整的
Agent-visible 状态机合同。与此同时，首轮示例只创建 `inspect`，后续 handoff 和 finish 示例却操作未创建的
`implement`；三个示例各自可解析，但不能在同一个 Map 上连续执行。

## 2. 修复

`taskspace_exec.description` 现在包含唯一 `Map lifecycle contract`，明确：

- `parents` 是依赖事实；Root 是直接 Work 子节点的已满足边界；
- 尚未启动节点根据非 Root parents 在 Waiting/Ready 间机械派生；
- Agent 可显式执行 Ready -> InFlight、Ready -> Completed、InFlight -> Completed；Tool action 可机械启动 Ready owner，
  Tool outcome 不自动完成节点；
- 同一个 Map update 不能把原本 Waiting 的子节点直接改出 Waiting；`update_and_work` 可先完成父节点，再通过 Tool action
  启动已派生为 Ready 的子节点；
- Finish、Root 和 reopen 的特殊生命周期保持原有实现。

Runtime 状态规则没有改变。全局约束和合法序列设计文档同步到现有真实实现，不再错误暗示每个节点都必须经过 InFlight。

三个 canonical 示例改为同一条路径：

```text
root -> inspect -> implement -> finish
```

它们依次执行初始化并检查、完成检查并开始实现、完成实现并显式关闭 Map。

## 3. 离线验收

- `cargo test -p codex-core taskspace_exec --lib`: 70 passed；
- 新测试把三个示例依次 preflight 到同一个 canonical Map，最终 Root、Implement、Finish 均为 Completed；
- declaration 测试确保生命周期合同只出现一次并覆盖关键转换；
- `cargo fmt --all -- --check` 和 `git diff --check` 通过；
- 缓存敏感面门禁免费 final-wire 验证通过，候选指纹为
  `e31a4ebf5d69087e02e3effe2b5a6e1b9b12716543a8e48487b5b4f6a5e7593e`；真实缓存基线未晋升。

工程修复完成。是否降低 Waiting frontier、冗余 InFlight 或请求成本仍需另行批准的真实运行验证；当前不能由离线测试推断。
