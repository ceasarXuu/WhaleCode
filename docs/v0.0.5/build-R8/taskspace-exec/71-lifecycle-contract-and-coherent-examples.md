# lifecycle 合同与连续示例修复

- Date: 2026-08-17
- Scope: `taskspace_exec` Agent-visible Tool description
- Live Whale run: 未执行

## 1. 问题

节点依赖、状态派生、Tool 启动和完成规则此前分散在顶层说明、sequence branch、字段说明和设计文档中，没有一个完整的
Agent-visible 状态机合同。与此同时，首轮示例只创建 `inspect`，后续 handoff 和 finish 示例却操作未创建的
`implement`；三个示例各自可解析，但不能在同一个 Map 上连续执行。

## 2. 修复

`taskspace_exec.description` 现在包含唯一 `Node state-machine contract`。它不是泛化提示，而是按照状态机的实际执行顺序明确：

- Root、Work、Finish 三种节点角色及其边界；
- Waiting、Ready、InFlight、Completed 四种状态分别意味着什么；
- Runtime 派生、Agent 显式更新、Tool dispatch 三类触发者各自能触发哪些转换；
- Map update 先于 Tool dispatch 提交，因此父节点完成后可在 `update_and_work` 中启动直接子节点；
- Tool 结果只记录 outcome，不完成 owner，也不解锁后代；
- Finish、Root 和 reopen 的特殊生命周期。

协议明确列出 Work 节点唯一允许的显式转换：

```text
waiting <-> ready                         Runtime 根据 parents 派生，仅限尚未启动节点
ready -> in_flight                        Agent state patch 或 Tool dispatch
ready -> completed                        Agent state patch
in_flight -> completed                    Agent state patch
```

其他显式状态转换全部非法。这样 Agent 无需从错误反馈或分散字段说明中反推状态机。

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
