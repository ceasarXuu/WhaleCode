# U16 localhost TaskSpace browser viewer

## 结论

`/taskspace` 与 `/task-show` 现在会打开当前 thread 的本地浏览器 viewer。viewer 只监听 `127.0.0.1` 随机端口，并每 2 秒调用 U15 的 `thread/taskspace/read`；退出 TUI、切换到另一 thread 的 viewer 或 server handle 被替换时，旧监听任务会被终止。

本段没有保存 Map 副本，也没有消费 notification 建立第二份 projection。`thread/taskspace/updated` 仍用于 app-server client 的 thread 定向，viewer 的展示权威始终是 canonical read response。

## 与旧实现的差异

旧页面约 400 行，并依赖 `bootstrapRequired`、节点派生 state、frontier count 等已从 U15 DTO 淘汰的 projection 字段。本次采用 222 行独立模块，只展示：

- schema、mode、Map ID 与 revision；
- root/work/finish 节点及 source refs；
- canonical edges；
- completion、block、reservation、result 与 terminal facts 的计数或表格。

因此 viewer 不重新引入旧 ActionMap projection、telemetry 或 provider trace。

## 安全与验证

- HTTP server 只接受 GET，提供 `/` 与 `/snapshot.json`，响应强制 `Cache-Control: no-store`。
- 启动 viewer 前先执行一次 typed read，线程未加载或 RPC 失败时不会打开空页面。
- `cargo check -p codex-tui`：通过。
- viewer path/canonical-field tests：2 passed。
- `cargo clippy -p codex-tui --all-targets -- -D warnings`：通过。
- 手写生产新增约 226 行；真实模型请求为 0。
