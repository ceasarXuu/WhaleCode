# R5-J6.6 Active 单份普通工具表达计划

- Created: 2026-07-12
- Updated: 2026-07-12
- Status: In Progress
- Owner: WhaleCode tools / native tool scheduler
- Prerequisite: R5-J6.5 complete，R5-J2 ordered barrier complete
- Blocks: R5-J7 implementation
- Risk: High

## 1. 问题

J6.5 已把 control 内 init/finish 两份 ordinary-action union 收敛为 `$defs` 中一份，并在 blank Map
隐藏顶层普通工具。但 active Map 仍同时发送：

```text
top-level ordinary tools
taskspace_control.$defs.ordinaryAction
```

这使 active request 的 non-message payload 约36.35 KB，Standard 约21.69 KB。额外约14.67 KB 是同一
ordinary capability 的第二次表达，占同进度 wire 增量约四分之三。

## 2. 边界决策

active 状态只保留顶层 ordinary tools 作为普通能力唯一表达。`taskspace_control` 只管理 Map 生命周期，作为与
普通工具并列的状态 barrier，不再承载普通工具参数或执行结果。

现有 J2 native ordered scheduler 已支持同一 response：

```text
taskspace_control state barrier
ordinary tool calls using latest binding
```

Runtime 按 Agent 声明顺序机械执行，不重排、不推断、不补动作。barrier 失败时沿用现有首错停止与 skipped
反馈。该能力足以替代 active `finish_then_actions` nested carrier。

blank Map 仍只暴露 bootstrap control；因为普通工具在硬状态初始化前不可直接执行，
`initialize_then_actions` 保留一份精确 ordinary-action schema。blank request 没有顶层重复。

## 3. Schema

### 3.1 Bootstrap control

只暴露：

- `initialize_then_actions`；
- 精确 ordinary-action `$defs` 一份。

### 3.2 Active control

只暴露：

- `finish_nodes`：一个或多个有序 nonterminal finish，每个 finish 原子建立 next binding；
- `finish_then_end`：可选 preceding finishes + terminal finish + Agent final candidate；
- `create_node`、`bind_node`、`block_node`、`read_output_ref` 等机械 Map 动作。

active control schema 不含 `actions`、`tool_name`、ordinary arguments 或 `$defs.ordinaryAction`。

删除 `finish_then_actions`，不保留兼容解析。普通工具由 Agent 在同一 response 中作为后续 sibling tool calls
声明，J2 scheduler 在 barrier 后执行。

## 4. 非目标

1. 不把所有普通工具收编到 control-only wrapper。
2. 不使用 generic arguments、摘要 schema 或跨 tool 非标准 `$ref`。
3. 不让 Runtime推断 finish 后应该调用什么工具。
4. 不新增 standalone finish 的后置惩罚或语义性 cadence gate。
5. 不改变 Standard 工具集合和调度路径。
6. 不在本阶段实施 J7 singular patch contract。

## 5. 实施与门禁

### J6.6-A：基线重复

并行执行3次当前 R5 `count-call-stack` right-only，统计错误 package 路径、错误 patch context 和其他低级动作。

退出条件：区分稳定机制问题与随机 Agent 路径，不因单次错误增加 Runtime 约束。

### J6.6-B：双形态 control schema

1. tools crate 提供 bootstrap control 与 active control 两个构造器。
2. blank named-choice request 只发送 bootstrap control。
3. active auto request发送 ordinary tools + lightweight active control。
4. schema单测证明 active control 不含 ordinary tool参数。

### J6.6-C：Handler 与 ordered barrier

1. typed args 删除 `FinishThenActions`，增加 `FinishNodes`。
2. handler只提交 finishes，不执行 nested ordinary action。
3. 复用 J2 response-local barrier执行后续 sibling ordinary calls。
4. 测试 `finish_nodes -> patch/test` 最新binding归属和失败停止。

### J6.6-D：收益验证

运行 `count-call-stack` Standard/R5 Docker paired sample。

门禁：

- 两侧 solved，public/hidden validator通过；
- active ordinary capability schema物理出现一次；
- R5 active non-message bytes显著低于J6.5约36.35 KB；
- 原始工具参数、结果和失败反馈不变；
- Map不坍缩，terminal extra request=0；
- request、input、cached/uncached、output和wall完整分账。

## 6. 基线结果

三次 current-J6.5 right-only 并行样本均 solved：

| Run | Requests | Tools | Wall | Input | Package-path error | Patch-context error | 其他低级动作 |
|---|---:|---:|---:|---:|---:|---:|---|
| run-1 | 7 | 10 | 19.02s | 81,399 | 0 | 0 | CLI未设置`PYTHONPATH`一次 |
| run-2 | 9 | 12 | 24.03s | 99,519 | 0 | 0 | 首个control参数malformed一次 |
| run-3 | 9 | 13 | 22.48s | 106,904 | 1 | 0 | 修复前pytest失败属于预期诊断 |

原始 J6.5 paired run 的 package-path error 与 patch-context error 均为1。合并观察后分别为2/4和1/4，
没有稳定复现；错误类型在不同run间变化。该证据不支持增加Runtime语义约束，只支持继续降低无收益的上下文/schema负担。

Evidence：

- `target/r5-j6-6-baseline-parallel/run-1/count-call-stack/20260712-064055-854`。
- `target/r5-j6-6-baseline-parallel/run-2/count-call-stack/20260712-064055-857`。
- `target/r5-j6-6-baseline-parallel/run-3/count-call-stack/20260712-064055-898`。

## 7. 完成矩阵

| Item | Code | Test | Runtime evidence | Status |
|---|---|---|---|---|
| 3-run error stability | benchmark artifacts | raw trace classification | 3 right-only | passed |
| bootstrap/active schemas | tools crate complete | unit/visibility passed | provider wire pending | code passed |
| finish lifecycle barrier | core handler/scheduler complete | 7 integration scenarios passed | rollout pending | code passed |
| paired benefit | Docker runner | Standard/R5 | observation report | pending |

已通过的工程回归：`codex-tools` 140 passed/1 ignored、TaskSpace control 11 passed、ordered
sequence 6 passed、action-map integration 7 passed、visibility 1 passed，以及 performance observation、cost
instrumentation、benchmark harness、skill validation 自测。
