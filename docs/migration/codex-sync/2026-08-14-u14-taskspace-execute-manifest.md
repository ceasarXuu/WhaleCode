# U14 extension runtime：active Map execute 写循环

- 日期：2026-08-14
- 状态：`verified`
- 范围：U14 第三原子段，仅恢复已有 active canonical Map 的 `execute` manifest
- 真实模型请求：0
- 手写生产代码：494 行新增、31 行删除；单个文件均小于 500 行

## 结果

TaskSpace 的 `taskspace_control` 已从只读工具扩展为已有 Map 上的原子写入口。完整 response 到达后，preflight 严格校验 control 位于首位、manifest 与所有 sibling 的数量/顺序/工具名一致、call id 唯一，并把 graph/node mutation 与 action reservation 组装成 canonical `ExecuteTransaction`。事务通过现有 `StateRuntime` CAS 落库后，工具 executor 才会被轮询；control handler 只返回已提交的 `TaskSpaceResponseCommitV1` receipt，不执行第二次写入。

每个 sibling 的 reservation 由 extension tool lifecycle 在完成、失败、阻断或取消后释放，并写入带 `is_error` 的 result ref。并行 sibling 释放使用 canonical store CAS 冲突重试，本地 runtime record 只向更高 store revision 前进，不持有异步锁跨持久化 await。manifest 不匹配或 domain transaction 拒绝时整批继续沿上一原子段的 zero-dispatch 路径关闭，且不产生部分提交。

## 当前合同

- `read_map`：必须是响应内唯一工具调用，不写状态。
- `execute`：必须是首个调用，且至少有一个 sibling；`actions[]` 与 sibling 一一对应。
- 支持的 mutation：`add_work_nodes`、`add_edges`、`remove_edges`、`complete_node`、`block_node`、`unblock_node`。
- reservation/action/result id 由 map、control call、response index 和 sibling call id 确定性生成。
- 未绑定 TaskSpace 的 Standard thread 继续完全放行，不暴露 `taskspace_control`。

## 明确未纳入

- Map 初始化与模式启用；
- terminal `finish_map` 与 `reopen_map`；
- app-server RPC/schema、TUI/viewer；
- 旧 `core/action_map`、session/provider wire sequence 或第二状态库。

这些边界继续留在 U14 后续小段及 U15/U16，避免为了恢复一个 active Map 写循环重新侵入 Codex 0.147 core。

## 验证

- `cargo test -p codex-taskspace-extension --lib`：38 passed；覆盖错误 manifest 零提交、execute receipt、reservation release 与 result attribution。
- `cargo test -p codex-state taskspace_maps_tests`：4 passed；覆盖 replay、owner、binding 与并发 CAS。
- `RUST_MIN_STACK=33554432 cargo test -p codex-core --test all tool_batch_preflight::rejected_complete_response_closes_call_without_dispatch`：1 passed；拒绝批次 executor dispatch 为 0。
- `cargo test -p codex-extension-api --test registry`：6 passed。
- `cargo clippy -p codex-taskspace-extension -p codex-state --all-targets -- -D warnings`：通过。
- `cargo fmt --all`：通过。

## 下一步

U14 下一原子段只恢复 Map 初始化/启用入口，使 Standard thread 能显式进入 TaskSpace；`finish_map`、`reopen_map` 是否同段纳入，在开工时按剩余生产代码预算和宿主 seam 重新核定。U15/U16 仍分别负责 RPC/schema 与 TUI/final-wire/cache，不前移。
