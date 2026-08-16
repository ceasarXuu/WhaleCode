# IC-02 自然历史结构拆分结果

- Status: Complete
- Date: 2026-08-17
- Scope: Provider wire 观测，不改变 Agent context、历史内容、Tool 执行或 Runtime 决策

## 实现

`natural_history` 保持原有总 section，同时新增固定的结构明细：

- `user_message`、`assistant_message`
- `client_tool_call`、`client_tool_output`
- `taskspace_exec_call`、`taskspace_exec_output`
- `provider_hosted_item`
- `reasoning_item`、`compaction_item`
- `other_history`

分类只读取 Responses 原生 `type`、`role`、`call_id` 和 Function Tool 名称。`function_call_output` 通过同一请求历史中已经存在的
`call_id` 机械关联其 call；不解析 arguments、output、reasoning 或自然语言。Web Search 和 Image Generation 作为各自原生
Provider item 整体计量，不拆分其内部 action。

## 完整性

- 每个进入 `natural_history` 的 wire item 恰好进入一个明细类别。
- 所有明细 bytes 之和必须等于 `natural_history.bytes`；Debug 构建中不闭合即断言失败。
- 明细只保存 count、bytes、估算 token 和内容 hash，不保存原文。
- 无法由原生结构确定的 item 进入 `other_history`，不靠关键词猜测归属。

## 验证

| 验证 | 结果 |
|---|---|
| mixed Responses history 十类覆盖 | 10/10，各一次 |
| history breakdown 与 natural history bytes | exact |
| `cargo test -p codex-core provider_wire_sections --locked` | 13 passed |
| 真实 Whale Agent run | 未执行 |

## 证据边界

已有 v1/v2 历史 trace 没有这些明细，不能追溯补造。IC-05 可先复算其总面积；新的真实双臂只有在 IC-A 全部通过后才会产生
可用于比较的 history breakdown。
