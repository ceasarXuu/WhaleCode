# U14 extension runtime：显式启用与初始化

- 日期：2026-08-14
- 状态：`verified`
- 范围：U14 第四原子段，仅恢复 TaskSpace 显式 mode gate 与 `initialize_and_execute`
- 真实模型请求：0
- 手写生产代码：321 行新增、52 行删除；单个生产文件均小于 500 行

## 结果

TaskSpace extension 现在持有一个轻量 `TaskSpaceService`，按 thread 注册弱引用 runtime，并提供显式 `set_enabled` seam。未绑定且未启用的 Standard thread 继续不暴露 `taskspace_control`、不注入 TaskSpace WorldState，也不改变 provider-visible tool 集合。宿主显式启用后，工具才出现；关闭后 turn refresh 不会自动把 mode 反转。U15 将直接复用该 service 接入 app-server mode RPC，本段不提前增加协议或 TUI 路由。

启用后的首个完整响应可以调用 `initialize_and_execute`。preflight 严格解析 Root、Work、Finish、完整 DAG 和 ordered action manifest，确定性生成 `map-{thread_id}`、reservation/action id，并通过现有 `StateRuntime` 的 `expected_store_revision=0` CAS 同时创建 canonical Map、owner binding 与首批 reservations。CAS 成功后 control handler 只返回提交 receipt；普通 sibling 沿上一原子段的 lifecycle 写回结果并释放 reservation。非法 DAG、manifest 不匹配、重复初始化或 store conflict 均在 executor dispatch 前拒绝，不产生 placeholder Map 或部分状态。

同时修正了 `execute.add_work_nodes` 的 wire/internal 边界：模型仍只提交 `node_id/goal`，内部 `source_refs` 由 runtime 构造，不要求模型伪造内部字段。

## 明确边界

- `TaskSpaceService` 已是 U15 可复用的 extension-owned mode seam，但 app-server 尚未持有和调用它。
- mode 本身不写入第二数据库；canonical Map 创建后仍是唯一持久化任务状态。
- Standard 默认关闭，已有/继承 canonical Map 在首次 hydrate 时自动恢复启用。
- 本段不包含 `finish_map`、`reopen_map`、extension event sink、RPC/schema 或 TUI/viewer。

## 验证

- `cargo test -p codex-taskspace-extension --lib`：39 passed；新增覆盖 Standard 默认隐藏、显式启用、初始化+首批 reservation 原子提交、receipt、release，以及关闭后 refresh 不反转。
- `cargo check -p codex-taskspace-extension -p codex-app-server`：通过，现有宿主可忽略 install 返回的 service，不改变行为。
- `cargo clippy -p codex-taskspace-extension -p codex-state -p codex-app-server --all-targets -- -D warnings`：通过。
- `cargo fmt --all`：通过。

## 下一步

U14 下一原子段评估并恢复 `finish_map`、`reopen_map` 与 canonical event emission；若三者合计超过单段 500 行生产预算，则优先按 terminal 与 reopen/event 分段。U15 仍负责把本段 service 接到版本化 app-server mode/read RPC，U16 负责 TUI 和 final-wire/cache。
