# Phase B5 CP-10：内层调用观测归属

- Date: 2026-08-10
- Scope: `ToolCallSource`、TaskSpace client dispatch、rollout trace/reducer
- Status: verified offline

## 1. 问题

TaskSpace 内部 client Tool 复用原生 `ToolCallRuntime` 时沿用了 `ToolCallSource::Direct`。执行行为没有错误，但 rollout trace
会把这些调用标记为模型顶层直接调用，导致同一内部 Action 无法从通用 Tool trace 机械关联到 outer Exec、`call_index`
和 Agent 声明的 owner node。

## 2. 修复

1. `ToolCallSource` 新增纯观测型 `TaskSpaceExec { outer_call_id, call_index, node_id }`；
2. TaskSpace dispatch 使用已有 request-local identity 和 Agent 已声明的 `node_id` 构造该来源，不推断、不补写；
3. rollout trace/reducer 原样保留这三个机械字段，并将 requester 归类为 `task_space_exec`；
4. trace 中的结果使用 CP-09 的实际 nested result，避免把 TaskSpace 内部结果误记为模型直调的 `ResponseInputItem`；
5. `Direct` 与 `CodeMode` 的既有身份和结果形状保持不变。

该来源只参与可观测性，不进入 Tool schema、Provider payload、Map、权限、sandbox、hook、并行锁或结果语义。

## 3. 验证

- `cargo test -p codex-rollout-trace`：38 PASS；
- `cargo test -p codex-core tool_dispatch_trace --lib`：4 PASS；
- `cargo test -p codex-core taskspace_exec --lib`：67 PASS；
- TaskSpace 的两个并行内部调用分别保留正确 outer call、call index 和 node；
- replay 后 Direct、Code Mode、TaskSpace 三种 requester 可区分，TaskSpace 没有伪造 model-visible call ID；
- TaskSpace nested result payload 被记录，原生并行和失败反馈测试保持通过。

## 4. 后续

CP-11 收敛 Hosted Tool 的分类事实和逐项核对。它只处理 Provider 已执行 output 与 Agent 声明归属的一致性，不复用
client requester 身份参与状态或执行决策。
