# Problem P-001: 旧版空 TaskSpace map 在 0.147 reader 中不可读
- Status: fixed
- Created: 2026-08-15 00:16
- Updated: 2026-08-15 00:33
- Objective: 让旧版合法的 `canonical_json="null"` TaskSpace placeholder 在升级后可安全读取并可由首次 canonical initialize 原位接管。
- Symptoms:
  - 旧数据库迁移成功，但 restore/RPC 读取绑定到空 map 的线程时反序列化失败。
- Expected behavior:
  - 空 placeholder 不应被当成 canonical map，也不应阻止同一 owner 初始化正式 canonical map。
- Actual behavior:
  - 当前 `decode_map` 强制反序列化为 `TaskSpaceMap`，JSON `null` 直接报错。
- Impact:
  - 0.147 升级后的旧 TaskSpace 线程无法恢复或重新初始化。
- Reproduction:
  - 构造旧版 schema row：合法 hash、`canonical_json="null"`、`map_revision=0`，再调用 `load_taskspace_map_for_thread`。
- Environment:
  - Linux，branch `whalecode-codex`，baseline `3761b8e1a`。
- Known facts:
  - 旧实现将 `Option<TaskSpaceCanonicalMap>` 直接序列化，因此可持久化 JSON `null`。
  - 当前 record 类型只表达已初始化的 canonical map。
- Ruled out:
  - schema migration 本身缺失：旧表可以被 0.147 runtime 打开，失败发生在 row decode。
- Fix criteria:
  - 旧空 row 的 load 返回 inactive，而不是 error。
  - 同 map_id、同 owner 的首次 CAS 能原位替换 placeholder，保留 binding 并产生可读 canonical record。
  - 非空 canonical map 和 hash validation 行为保持不变。
- Current conclusion: placeholder 被读取为 inactive，同 owner 首次 canonical CAS 可原位接管，真实 legacy migration fixture 已通过。
- Related hypotheses:
  - H-001
- Resolution basis:
  - H-001；E-001、E-002、E-003
- Close reason:
  - 原始 legacy fixture 症状已由 fix-validation 消除

## Hypothesis H-001: reader 把旧版 Optional map 错当成必有 map
- Status: confirmed
- Parent: P-001
- Claim: `decode_map` 的非可选反序列化与旧 store 的 Optional JSON 合同不兼容，并且普通 INSERT-on-conflict 使合法 placeholder 无法被初始化覆盖。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - reviewer 与主 Agent 分别从旧写路径和当前读/CAS 路径得到相同机制。
- Falsifiable predictions:
  - If true: JSON `null` 通过 hash 检查后会在 `serde_json::from_str::<TaskSpaceMap>` 失败；expected revision 0 的 INSERT 会因既有 placeholder 冲突。
  - If false: 当前 decoder 能返回 inactive，且首次 canonical initialize 能成功替换旧 row。
- Diagnostic evidence plan:
  - Prediction or clause under test: 旧 writer 可写 `null`，当前 decoder/CAS 无 placeholder 分支。
  - Signal: 旧提交与当前源码的类型、SQL 和反序列化目标。
  - Capture method: `git show` 与当前源码只读检查；随后以 state runtime 回归测试复现。
  - Event name or marker:
    - none
  - Correlation keys:
    - map_id and owner_thread_id
  - Differentiates from:
    - migration 没有建表或 hash 损坏
  - Supports if:
    - 旧 writer 序列化 Option，当前 reader 解析非可选 map，当前 INSERT 不替换 placeholder。
  - Refutes if:
    - 旧 writer 不可能写 null，或当前 reader/CAS 已有兼容分支。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: closed
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 旧生产 writer 可写 JSON null
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: commit `17424eac8`, `state/src/runtime/taskspace_maps.rs` and `core/src/action_map/runtime/state.rs`
- Prediction or plan link:
  - H-001 的旧 writer Optional 合同预测
- Matched signal:
  - `serde_json::to_string(&request.canonical_map)`，且 runtime test 明确允许 `canonical_map_for_store() == None`
- Correlation keys:
  - map identity
- Raw content:
  ```text
  canonical_map: Option<TaskSpaceCanonicalMap>
  let canonical_json = serde_json::to_string(&request.canonical_map)?;
  ```
- Interpretation: 旧版合法数据域包含 canonical JSON `null`。
- Time: 2026-08-15 00:16

## Evidence E-003: legacy migration placeholder 可读取并原位初始化
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `just test -p codex-state repairs_legacy_whale_taskspace_migration_collision_and_preserves_data`
- Prediction or plan link:
  - P-001 的全部 fix criteria
- Matched signal:
  - legacy revision-0/null row load 为 inactive；首次 CAS Applied；reload 保留 Owner binding 与 canonical map
- Correlation keys:
  - empty-map and empty_owner
- Raw content:
  ```text
  test migrations::tests::repairs_legacy_whale_taskspace_migration_collision_and_preserves_data ... ok
  ```
- Interpretation: 原始旧库状态通过真实 migration repair 路径恢复，不依赖新库伪造状态。
- Time: 2026-08-15 00:33

## Evidence E-002: 当前 reader 与首次写入均无 placeholder 兼容
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/state/src/runtime/taskspace_maps.rs`
- Prediction or plan link:
  - H-001 的当前 reader/CAS 预测
- Matched signal:
  - `serde_json::from_str::<TaskSpaceMap>`；revision 0 使用 `INSERT ... ON CONFLICT DO NOTHING`
- Correlation keys:
  - map_id and expected_store_revision=0
- Raw content:
  ```text
  let map: TaskSpaceMap = serde_json::from_str(&json)?;
  ON CONFLICT(map_id) DO NOTHING
  ```
- Interpretation: null row 必然 decode 失败，且即使被视作 inactive 也会阻止首次初始化。
- Time: 2026-08-15 00:16
