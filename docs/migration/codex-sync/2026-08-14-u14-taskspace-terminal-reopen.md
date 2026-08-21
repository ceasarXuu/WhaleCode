# U14 extension runtime：terminal 与 reopen

- 日期：2026-08-14
- 状态：`verified`
- 范围：U14 第五原子段，仅恢复 `finish_map` 与 `reopen_map`
- 真实模型请求：0
- 手写生产代码：249 行新增、24 行删除；单个生产文件均小于 500 行

## 结果

`finish_map` 现在要求 control call 独占完整响应，并在 executor dispatch 前严格解析 expected revision、finish node、最终完成节点和 exact summary。它沿用 canonical `FinishMap` transaction 与现有 state CAS，在一个 revision 中写入最终 completion facts 和 terminal record；summary 原文继续作为 canonical `summary_ref`，action id 沿用 control call id。携带 sibling 的终结请求在 preflight 阶段拒绝且零提交。

`reopen_map` 复用 `execute` 已验证的 ordered action manifest：每个声明必须与后续 sibling tool call 一一对应且顺序、工具名、call id 均有效。canonical transaction 在一个 revision 中归档原 terminal、追加 work nodes/edges 并建立 reservations；control handler 返回 commit receipt，既有 tool lifecycle 在 sibling 完成后继续写 result ref 并释放 reservation。

runtime 的已有 active-map 提交入口被收敛为带 action/operation 参数的单个 `commit_control`，没有增加第二套 CAS、terminal 状态或 handler 路由。Standard 未绑定/未启用路径保持不变。

## 验证

- `cargo test -p codex-taskspace-extension --lib`：40 passed。
- 新增端到端覆盖：finish 携 sibling 零提交；finish 原子关闭；receipt；reopen terminal history；ordered reservation；sibling 完成后 release 与 revision 推进。
- `cargo clippy -p codex-taskspace-extension --lib --tests -- -D warnings`：通过。
- `cargo fmt --all`、`git diff --check`：通过。

## 边界与下一步

- 本段不增加 app-server RPC/schema、TUI/viewer 或 core/session/provider 专用分支。
- canonical terminal 已可持久化并随 read/WorldState 投影；面向宿主的 extension event emission 仍是 U14 最后一个原子段。
- U15 继续通过 extension-owned service 接 app-server read/mode 与版本化事件协议。
