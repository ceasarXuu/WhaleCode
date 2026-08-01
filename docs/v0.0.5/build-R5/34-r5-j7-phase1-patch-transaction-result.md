# R5-J7.1 Patch Prepare/Commit结果

- Date: 2026-07-13
- Status: Complete
- Scope: shared `codex-apply-patch` validation atomicity、precondition、best-effort rollback和结构化commit failure
- Related: `18-r5-single-patch-carrier-contract-plan.md`、`33-r5-j7-phase0-evidence-and-decisions.md`

## 1. 结果

`apply_patch`不再逐hunk边验证边写。生产路径现在是：

```text
parse patch
  -> prepare every operation and pre-image (read-only)
  -> validate every source/destination/precondition again (read-only)
  -> commit prepared operations
  -> on I/O failure, rollback current + committed operations in reverse order
```

所有parse、missing file、context mismatch、directory、symlink、parent、duplicate/collision和stale precondition失败
都发生在首个文件副作用前。Standard与TaskSpace继续共享同一crate和runtime handler，没有TaskSpace专用patch分支。

## 2. 模块边界

| Module | Responsibility | Lines |
|---|---|---:|
| `apply-patch/src/transaction.rs` | prepare、precondition、commit orchestration | 473 |
| `transaction/rollback.rs` | reverse best-effort restore与parent cleanup | 107 |
| `transaction/error.rs` | structured `PatchCommitError` | 40 |
| `transaction/tests.rs` | fault injection filesystem与rollback tests | 231以内 |

原`lib.rs`只保留解析、公共入口、内容计算和CLI summary。新自有模块均低于500行。

## 3. Validation合同

prepare阶段：

- 解析全部hunk并解析绝对路径；
- 读取所有existing source/destination pre-image；
- 计算全部update目标正文；
- 检查missing/delete/update、directory、symlink和non-directory parent；
- 拒绝同一patch内重复或source/destination冲突路径；
- 拒绝move source等于destination；
- 记录commit可能创建的parent目录。

prepare完成后再次读取所有pre-image；若文件在prepare/commit之间变化，以
`File changed after patch preparation`失败，且不执行commit。

Symlink在J7.1中明确拒绝。当前`ExecutorFileSystem`不能恢复link target和metadata，允许symlink mutation会让
rollback成功声明失真。后续如需支持，必须先扩展filesystem contract，不增加silent special case。

## 4. Commit与Rollback合同

commit仍按Agent patch中的操作顺序执行，不合并、不重排。I/O失败时：

1. 尝试恢复当前可能部分执行的operation；
2. 逆序恢复已提交operation；
3. 对新增文件执行remove，对覆盖/更新/删除恢复pre-image；
4. 尝试移除本patch创建且仍为空的parent目录；
5. 返回完整`PatchCommitError`。

错误字段：

```text
cause
committed_paths
pending_paths
rollback_restored_paths
rollback_failed_paths
rollback_status=best_effort_restored|best_effort_partial
```

这不是跨文件事务声明。文件权限、时间戳等metadata不在现有trait中，底层写入故障也不能由应用证明原子；
`best_effort`命名和failed paths忠实保留该边界。

## 5. 行为变化

| Case | Before | J7.1 |
|---|---|---|
| valid add + later missing update | add残留 | workspace不变 |
| valid update + later missing delete | update残留 | original保持 |
| duplicate/colliding paths | 顺序副作用或后序失败 | prepare拒绝，零副作用 |
| move source == destination | 可能写后删除 | prepare拒绝 |
| parent是普通文件 | commit阶段失败 | prepare拒绝 |
| symlink target | 行为依赖FS实现 | prepare明确拒绝 |
| second write I/O failure | 前序写入残留 | reverse rollback并报告 |
| rollback failure | 模糊patch failure | `best_effort_partial` + failed paths |

保留的既有行为：add覆盖existing file、move覆盖existing destination、missing parent创建、stdout成功summary、
stderr错误、权限/沙箱/hook和freeform/function两种入口。

旧partial-success fixture未直接删除：旧expected正文安全移入`tests/fixtures/legacy/`，active scenario改名为
`015_validation_failure_zero_side_effects`且expected为空。

## 6. 验证

| Gate | Result |
|---|---|
| `cargo test -p codex-apply-patch` | 64 lib + 22 CLI/scenario passed |
| validation zero-side-effect cases | passed |
| add/move overwrite compatibility | passed |
| symlink/parent/collision cases | passed |
| write/remove fault injection | passed |
| rollback failure reporting | passed |
| `cargo test -p codex-core apply_patch --lib` | 16 passed |
| `cargo build -p codex-cli --bin whale --locked` | passed |
| `cargo fmt -p codex-apply-patch` | passed；仅既有stable rustfmt warning |

## 7. 下一门禁

J7.2开始修改model-visible TaskSpace bootstrap schema和typed parser：carrier中patch成为唯一slot，ordinary action
union排除patch；同时建立共享`ToolSequenceManifest`类型。J7.3之前manifest只计算/测试，不产生production reject。
