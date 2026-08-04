# TS-04：TaskSpace Control 统一 Router Seam 验证结果

> **已封存证据（2026-08-05）**：旧容器计划停止；统一 Router seam 结果保留供 TaskSpace Exec 复用，不代表旧方案仍有效。

- Date: 2026-08-03
- Status: Verified
- Code evidence: `148406cde`
- Scope: 停点 1，仅验证 `taskspace_control` 能否作为普通 Tool 经统一 Router 生命周期触发唯一 Map 事务
- Production behavior: 未切换
- Whale Agent run: 未执行，API 成本为 0

> 2026-08-04 复核：本验证只涉及 `taskspace_control` 的统一 Router/Map transaction seam，不依赖 hosted adapter、
> Ready frontier 或 Tool outcome 推导节点状态。产品基线纠偏后，本结论仍完整有效。

## 1. 验证结论

停点 1 已通过，不需要修改产品路线。

未来序列容器可以把 Agent 声明的容器身份和节点归属作为 Runtime 内部 invocation metadata，随原生
`ToolInvocation` 进入现有 `ToolRouter -> ToolRegistry -> ToolHandler`。`taskspace_control` 的 handler 可以从同一份
metadata 获得 Work bindings，并通过 Session 触发一次 canonical Map prepare/commit。普通 Tool 的名称、schema、原生
参数和 handler 均不需要知道 Map、节点或容器。

这条 seam 同时满足三个停点条件：

1. control 经过真实 Router/Registry/handler 生命周期，未建立 control 专属 dispatcher；
2. Map 只由 handler 依赖触发一次 canonical transaction，外层 sequence runtime 不直接提交；
3. Work 归属只来自 Agent 声明的容器 metadata，不从 control 参数复制，也不使用全局临时 registry。

## 2. 当前问题与验证结构

当前生产路径在 response preflight 中先调用 `prepare_taskspace_response()`，随后真正的
`TaskSpaceControlHandler` 会拒绝 initialize/execute/reopen。也就是说，当前 control 虽然在 Tool Registry 注册，却没有
对这些动作承担普通 Tool 的执行职责；Map 提交发生在 Router 之前的旁路。

本次新增的是未接生产入口的 seam probe：

```text
Agent 声明的未来容器
  -> Runtime 解码为 TaskSpaceSequenceInvocation
  -> 原生 ToolCall + runtime-only ToolCallSource
  -> ToolRouter
  -> ToolRegistry
  -> 测试 control handler
  -> Session canonical Map transaction（一次）
```

探针故意在旧 `taskspace_control.actions[]` 中放入与容器 metadata 冲突的 `legacy_copy`，最终 reservation 仍严格采用
metadata 中的 `probe_work`。这证明旧 actions 副本不是新路径的绑定事实来源。

## 3. 验证期间发现并收敛的边界

### 3.1 容器位置不是第二套调用序号

旧 sibling 路径要求 Work 的 `call_index` 必须恰好是 `0..N`。序列容器中，control 可能位于第 0 项，首个 Work 的真实
容器位置因此是 1。若继续把 Work 重新编号为 0，就会产生“容器位置”和“Map 位置”两套身份。

Map 事务现改为接受严格递增的非负容器位置，并继续拒绝重复或倒序位置。旧路径原有的 `0..N` 仍是合法子集，未增加
兼容分支或第二个 index 字段。

### 3.2 外层事件身份与内部 control 身份必须分开

provider history 中可见的是外层容器调用；内部 control item 使用 `outer_call_id/item_id` 作为唯一调用身份。Map 操作和
reservation 使用内部 control 身份，初始化证据引用则使用 provider 可见的外层事件身份。Session 只增加一个显式传入
两种身份的内部入口，旧生产调用继续把同一 id 传给两者，因此当前行为不变。

### 3.3 Trace 保留统一调用事实

内部 item 的 dispatch trace 以 `outer/item` 为记录键，同时把 `model_visible_call_id` 指向外层容器 id，并继续保存原始
handler 结果。它没有伪造第二个模型调用，也没有把内部 item 暴露成新的顶层 provider Tool。

## 4. 本地证据

| 验证 | 结果 | 主要断言 |
|---|---:|---|
| `cargo test -p codex-core taskspace_sequence` | 3 passed | control 只 dispatch 一次；Map 只产生一份 reservation；冲突旧 actions 不生效 |
| `cargo test -p codex-core action_map::runtime::transactions::tests` | 9 passed | 非零递增 item index 合法；重复/倒序拒绝；旧事务保持通过 |
| `cargo test -p codex-core tools::sequence::` | 41 passed | 当前生产 sibling sequence 回归无失败 |
| `cargo test -p codex-core tools::tool_dispatch_trace::tests` | 4 passed | Direct、Code Mode 与容器内部 item 的 trace 归属均正确 |
| `cargo fmt --all -- --check` | PASS | 仅有项目既有 nightly formatter 配置警告 |
| `python3 scripts/cache-regression/check_cache_regression_gate.py --source index` | PASS | 免费 final-wire 指纹 `6835851b...eff839` |

## 5. 对正式工程计划的约束

重写后正式计划的 TS-15 必须沿此 seam 接入，不能重新解释为另一条实现：

1. 正式 `TaskSpaceControlHandler` 消费 Runtime 传入的 sequence invocation metadata；
2. batch preflight 只验证并生成计划，不提交 Map；
3. handler 通过 Session 的单一入口完成 canonical Map prepare/commit；
4. 外层 scheduler 只消费 handler 返回的执行元数据，不复制 control 参数或再次提交 Map；
5. 普通 Tool schema、原生 input 和 handler 保持无侵入；
6. 不增加 transient global registry、隐式 current node 或 Runtime 主动绑定策略。

当前探针仍复用了旧 control parser，以便只验证 Router/Map seam；它不代表 `actions[]` 会进入正式容器协议。重写后
容器合同、decoder/reconciler、preflight 和生产原子切换分别属于 TS-06、TS-12、TS-13、TS-19；纯 Map control 由
TS-15 沿本文 seam 接入。生产入口在 TS-19 前继续使用旧路径，不能同时启用新旧两条 Map 提交路线。

## 6. 停点判定

停点 1 的工程可行性风险已消除：统一 Router seam 可行，当前 TS-15 状态为 `planned`。本次没有
启动 Phase B、没有接入生产容器、没有运行真实 Whale Agent，也没有自动推进其他 R8 已知问题。
