# U15：TaskSpace service read seam

- 日期：2026-08-14
- 状态：`verified`
- 范围：U15 第一原子段，仅建立 app-server 可持有的 service read/injection 边界
- 真实模型请求：0

## 结果

`TaskSpaceService` 现在可按 thread 读取 runtime 的 enabled 状态与刷新后的 canonical record；读操作仍经过同一 `TaskSpaceStore`，不建立缓存副本或第二权威。extension 新增 `install_with_service`，允许 app-server 在构建 registry 前创建并保留同一个 service；原 `install` API 保持兼容。

U14 的 event sink 项经源码核定后并入 U15：0.147 sink 只接收已经存在的 `codex_protocol::EventMsg`，而当前主线没有通用 extension payload。事件类型、sink emission、app-server notification 和 JSON/TS schema 必须在同一版本化 wire 单元落地，不能先发射不可消费事件。

## 验证与边界

- `cargo test -p codex-taskspace-extension --lib`：40 passed，新增验证 service read 会刷新 canonical record 且保持显式关闭状态。
- 不修改 core/session/provider，不增加 RPC、schema、TUI 或第二 runtime。
- 下一段恢复 `thread/mapRuntimeMode/set` 与 `thread/taskspace/read` 的最小版本化 DTO 和 app-server adapter。
