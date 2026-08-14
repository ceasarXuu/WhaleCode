# U15 TaskSpace canonical update event 与通知

## 结论

U15 第三原子段已恢复 TaskSpace 的版本化更新事件和 App Server 通知。事件只在候选 canonical record 的 revision 严格大于已存 revision、且 CAS 持久化成功后发射；重复结果、拒绝事务和旧 revision 均不会产生通知。

本段没有恢复旧 `core/session/provider` 专用分支，也没有把完整 Map 复制进事件。canonical store 仍是唯一任务状态权威，客户端收到通知后通过 `thread/taskspace/read` 获取 `taskspace-snapshot-v1` 完整快照。

## 合入边界

- protocol 增加 `TaskSpaceUpdatedEvent`，字段仅含 `thread_id`、`turn_id`、`map_id`、`revision`、`operation`。
- `ext/taskspace` 通过已有 `ExtensionEventSink` 发射事件，事件 ID 固定为 `taskspace:{map_id}:{revision}`。
- 覆盖 `initialize_and_execute`、`execute`、`finish_map`、`reopen_map` 和 `action_release` 五类成功提交。
- App Server 优先将事件写入 thread listener command FIFO，保持与 turn/resume 通知的顺序；listener 不可用时沿用现有 extension sink 的异步通知退路。
- 新增 experimental `thread/taskspace/updated` notification，并重生成 JSON、TypeScript 和 precomputed schema exports。
- rollout 将该事件视为 transient：canonical TaskSpace journal/store 已承担持久化和 replay，避免形成第二份 durable 状态。

## 行为合同

```json
{
  "method": "thread/taskspace/updated",
  "params": {
    "threadId": "thr_123",
    "turnId": "turn_456",
    "mapId": "map_789",
    "revision": 2,
    "operation": "finish_map"
  }
}
```

通知用于表达“哪个 canonical revision 已经提交”，不保证携带 Map 内容。客户端应以 `threadId + mapId + revision` 去重；需要展示或恢复时调用 read RPC。

## 验证

- `cargo test -p codex-taskspace-extension --lib`：40 passed。
- App Server extension event FIFO 定向测试：1 passed。
- TaskSpace protocol wire 定向测试：1 passed。
- schema fixtures：6 passed、1 ignored。
- `cargo clippy -p codex-taskspace-extension -p codex-app-server-protocol -p codex-app-server --all-targets -- -D warnings`：通过。
- cache regression index gate：通过。

全部验证均为本地确定性测试，真实模型请求与费用均为 0。
