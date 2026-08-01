# R7.1 TaskSpace Map Store 运行手册

## 适用范围

本手册用于 R7.1 canonical Map 的本地开发、测试和故障定位。当前事实源是
`$CODEX_HOME/state_5.sqlite` 中的 TaskSpace 表；rollout 只保存对话与审计信息，不负责重建 Map。

canonical schema 必须是 `taskspace-canonical-map-v2`。产品不读取或迁移旧 TaskSpace Map 数据。

开发构建的二进制名是 `whale`，不是 `codex`：

```bash
cd third_party/codex-cli/codex-rs
cargo build -p codex-cli --bin whale --locked
```

## 快速检查

先停止正在使用同一 `CODEX_HOME` 的 WhaleCode 进程，再设置数据库路径：

```bash
export CODEX_HOME="${CODEX_HOME:-$HOME/.codex}"
export WHALE_STATE_DB="$CODEX_HOME/state_5.sqlite"
test -f "$WHALE_STATE_DB"
```

查看 Map、owner、Store revision、Map revision 和终态标记：

```bash
sqlite3 -header -column "$WHALE_STATE_DB" \
  'select map_id, owner_thread_id, canonical_schema_version, store_revision, map_revision, terminal, updated_at from taskspace_maps order by updated_at desc limit 20;'
```

查看指定 Map 的完整 canonical JSON：

```bash
export TASKSPACE_MAP_ID="MAP_ID"
sqlite3 \
  -cmd '.parameter init' \
  -cmd ".parameter set :map_id '$TASKSPACE_MAP_ID'" \
  "$WHALE_STATE_DB" \
  'select canonical_json from taskspace_maps where map_id = :map_id;'
```

通过产品诊断入口导出同一份 canonical Map：

```bash
target/debug/whale debug taskspace-map \
  --thread-id THREAD_ID \
  --output /tmp/taskspace-map.json
```

导出协议为 `TaskSpaceMapExportR7V2`。重点字段是 `canonical_map`、`canonical_sha256`、
`store_revision`、`map_revision` 和 `terminal`；线程关系不包含节点游标或 lease。

重点核对：

- `terminal_record`：当前关闭事实；非空表示 Map 已关闭。
- `terminal_history`：此前关闭事实；reopen 后原 `terminal_record` 必须进入这里。
- `completion_records`：只能包含 Work node，不得包含 Root 或 Finish。
- `action_reservations`：只表示尚未释放的原生 Tool 调用。
- `revision`：必须与表列 `map_revision` 一致。

## 重启验证

1. 记录目标 `map_id`、`canonical_sha256`、`store_revision` 和 `map_revision`。
2. 完全退出 WhaleCode。
3. 使用相同 `CODEX_HOME` 重新启动并恢复原线程。
4. 再次查询同一 Map。

必须满足：

- `map_id` 不变。
- canonical facts 直接从 Store 恢复，不依赖 rollout replay。
- 未发生新动作时，Map revision 和 canonical hash 不变。
- 当前 terminal、terminal history、Work completion/result/evidence 均保持。

对应自动测试：

```bash
cd third_party/codex-cli/codex-rs
cargo test -p codex-state taskspace_map_survives_state_runtime_restart --lib
cargo test -p codex-core runtime_close_reopen_close_preserves_one_map_and_terminal_history --lib
cargo test -p codex-cli --test debug_taskspace_map
```

## reopen 故障定位

`reopen_map` 只接受 `closed -> active`：

1. 确认 `terminal_record` 当前非空。
2. 确认提交的 `expected_revision` 等于 canonical revision。
3. 确认请求声明至少一个新 Work、至少一条 edge、至少一个 `actions[]` 项。
4. 确认同一响应中存在数量、顺序和 Tool 名完全匹配的原生 sibling calls。
5. 确认新节点接入现有 Root 到 Finish 的 DAG，且没有环、孤立节点或重复边。

成功后必须同时出现：

- 同一 `map_id`。
- 原 `terminal_record` 追加到 `terminal_history`。
- 当前 `terminal_record` 清空。
- 新 Work、edges 和 reservations 在同一 revision batch 提交。
- 旧 Work completion/result/evidence 不变。

失败时优先读取 `TaskSpaceControlResultV2` 的 `canonical_revision`、`state_commit` 和 violations。不要根据自然语言
猜测 Runtime 应该选择哪个节点，也不要恢复 `rework_node` 或 current-node 语义。

Store commit 日志中的一致性字段为 `mapRevision` 和 `canonicalSha256`。若观测脚本仍读取
`graphRevision` 或 `snapshotSha256`，说明它仍在使用已删除的旧导出合同。

## 实验 Store 重置

只在明确需要丢弃实验 TaskSpace 数据时执行。先退出 WhaleCode并创建可恢复备份：

```bash
backup="$WHALE_STATE_DB.taskspace-reset-$(date +%Y%m%d-%H%M%S).bak"
cp --reflink=auto "$WHALE_STATE_DB" "$backup"
echo "$backup"
```

随后只清空 TaskSpace 三张表，不删除整个 State DB：

```bash
sqlite3 "$WHALE_STATE_DB" <<'SQL'
PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;
DELETE FROM taskspace_map_commits;
DELETE FROM taskspace_map_bindings;
DELETE FROM taskspace_maps;
COMMIT;
SQL
```

验证：

```bash
sqlite3 "$WHALE_STATE_DB" \
  'select (select count(*) from taskspace_maps), (select count(*) from taskspace_map_bindings), (select count(*) from taskspace_map_commits);'
```

预期输出为 `0|0|0`。不要删除 `state_5.sqlite`，否则线程、会话和其他 State 数据也会丢失。

## 相关门禁

```bash
pwsh scripts/taskspace-benchmark/test-r7-five-layer-contracts.ps1 -Phase A2-B5
cd third_party/codex-cli/codex-rs
cargo test -p codex-core action_map::rooted_dag --lib
cargo test -p codex-core tools::sequence::tests --lib
cargo test -p codex-state taskspace --lib
```
