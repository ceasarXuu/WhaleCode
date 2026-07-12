# R5-J6.7.1 无损事件合同实施结果

- Date: 2026-07-12
- Status: Complete
- Commit: `75f4db2`

## 1. 实现

新增 `action_map/event_store.rs`，定义最小 `TaskSpaceEvent`：

- 全局单调 `sequence`。
- `Root` / `Node(id)` owner。
- 机械 `event_type`、原始 role、call_id、parent_call_id。
- 唯一 `raw_payload`，不生成 summary 或 excerpt。
- sidecar保存 `provider_item_id` 与 `tool_success`，修复原生 `ResponseItem` serde 明确跳过这两个
  internal字段的问题。
- `GhostSnapshot` 与 `Other` 显式拒绝，不做 silent fallback。

codec当前没有production caller，不写shadow events，也没有双写开关。

## 2. Round-trip矩阵

| 类型 | 覆盖字段 | 结果 |
|---|---|---|
| Message | id/role/text/image/detail/end_turn/phase | exact |
| Reasoning | id/summary/content/encrypted_content | exact |
| LocalShellCall | id/call_id/status/action/cwd/timeout | exact |
| FunctionCall | id/name/namespace/raw arguments/call_id | exact |
| ToolSearchCall/Output | id/call_id/status/execution/JSON tools | exact |
| FunctionCallOutput | call_id/text/content items/image/success/ref/truncation text | exact |
| CustomToolCall/Output | id/status/name/input/call_id/success | exact |
| MCP normalized output | structured FunctionCallOutput content/image/success | exact |
| WebSearchCall | id/status/action | exact |
| ImageGenerationCall | id/status/prompt/result | exact |
| Compaction | encrypted_content | exact |
| GhostSnapshot/Other | 不属于task semantic event | explicit error |

测试：

```text
cargo test -p codex-core action_map::event_store --locked -- --nocapture
3 passed, 0 failed

cargo build -p codex-cli --bin whale --locked
passed
```

## 3. Docker production no-change验证

Run root：`target/r5-j6-7-1-live2`。

| Sample | Mode | Result | Requests | Input | Cached | Wall | Map |
|---|---|---|---:|---:|---:|---:|---|
| count-call-stack | Standard | solved | 5 | 35,553 | 33,152 | 9,352ms | none |
| count-call-stack | R5 | solved | 9 | 72,855 | 63,872 | 19,400ms | 3 nodes, open=0 |
| large-output-ref-smoke | Standard | solved | 6 | 43,027 | 40,704 | 11,068ms | none |
| large-output-ref-smoke | R5 | solved | 7 | 56,473 | 53,376 | 16,020ms | 2 nodes, open=0 |

`large-output-ref-smoke` R5生成1条 `output_ref.created`，`protected_miss=0`。本阶段没有production
行为变更，因此这些数据只证明无回归，不声明性能收益。

## 4. 操作经验

1. benchmark runner只读取进程环境，不自动加载`.env.local`。正式运行需先在当前shell中导出，不能把key
   放到argv、Dockerfile或artifact。
2. Bash调用PowerShell的`string[]`参数时，逗号分隔字符串会被当成一个配置值。本项目container contract
   已自动追加plugins/skills配置，直接使用默认`ConfigOverride`即可，避免把多项拼成一个TOML值。
3. 首次credential preflight与首次错误ConfigOverride运行均未进入有效Agent样本，已保留为
   `invalid_harness`诊断，不计入阶段结果。

## 5. Phase gate

- 已审计event type往返：100%。
- role/order/call_id/arguments/output/success/content：fixture exact。
- unsupported：显式失败。
- production shadow dual-write：0。
- focused/large-output Docker correctness：4/4 solved。

J6.7.1达到退出条件，允许进入J6.7.2原子切换。
