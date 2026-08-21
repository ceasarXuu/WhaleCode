# U15：TaskSpace read/mode RPC 与 schema

- 日期：2026-08-14
- 状态：`verified`
- 范围：U15 第二原子段，恢复 app-server read/mode API 与生成 schema
- 真实模型请求：0
- 手写生产代码：约 390 行新增；单个生产文件均小于 500 行

## 结果

app-server 恢复旧客户端 method 名 `thread/mapRuntimeMode/set` 与 `thread/taskspace/read`，继续作为 experimental API。`standard` 是默认 mode，`taskspace` 必须显式设置；mode set 和 read 都返回 `taskspace-snapshot-v1`。snapshot 只包含 mode 与 U13/U14 canonical Map 的版本化 DTO，不恢复旧 `ActionMapSnapshot` 中 telemetry、sentinel、trace 等已淘汰投影。

app-server 在进程级创建并持有同一个 `TaskSpaceService`，extension registry 通过 `install_with_service` 注册 runtime。adapter 先确认线程已加载，再调用 service set/read；没有给 `CodexThread`、core session 或 provider 增加 TaskSpace 方法，也没有第二个 runtime/cache。

JSON Schema、TypeScript 类型以及 stable/experimental precomputed exports 已按仓库实际 Python wrapper 重生成。仓库 `just write-app-server-schema` 当前仍指向已不存在的 Rust bin，本段没有顺带修改上游脚手架，避免扩大融合范围。

## 验证

- protocol DTO wire test：`schemaVersion`、`standard/taskspace`、nullable map 通过。
- app-server adapter mapping test：canonical map id、nodes、revision 与 mode 通过。
- schema fixture tests：6 passed，stable/experimental precomputed exports 均一致。
- `cargo check -p codex-app-server-protocol -p codex-app-server`：通过。
- `cargo clippy -p codex-app-server-protocol -p codex-app-server --all-targets -- -D warnings`：通过。
- `cargo fmt --all`、`git diff --check`：通过。

## 下一步

U15 下一原子段增加最小版本化 TaskSpace commit notification、`codex_protocol::EventMsg`、extension sink emission 与 app-server fanout，并同步重生成 schema。TUI/viewer 仍属于 U16。
