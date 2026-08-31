# Problem P-001: Codex 0.151 substrate CLI 回归
- Status: fixed
- Created: 2026-09-01 02:15
- Updated: 2026-09-01 02:44
- Objective: 消除 0.151 substrate cutover 引入的 CLI 测试回归，同时保持 Whale identity、现有数据库兼容和既有产品语义。
- Symptoms:
  - `cargo test -p codex-cli --lib --tests --no-fail-fast` 有 8 个失败 target。
- Expected behavior:
  - CLI 测试使用 Whale 二进制身份，fresh state DB 可迁移，已有 0.149 Whale DB 可无损升级。
- Actual behavior:
  - fresh state DB 因重复 migration 0051 初始化失败；部分 0.151 测试仍查找或断言 `codex`；其余网络/sandbox 用例在初始化链失败后没有达到预期请求或命令。
- Impact:
  - 0.151 substrate 尚不可提交，CLI、app-server、doctor、TaskSpace debug 和 sandbox 测试受影响。
- Reproduction:
  - `cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --lib --tests --no-fail-fast`
- Environment:
  - Linux；分支 `whalecode-codex`；目标 Codex `rust-v0.151.0`；production vendor cutover 未提交。
- Known facts:
  - `cargo check -p codex-cli --tests` 已通过。
  - 失败集中为 migration version 冲突、测试二进制身份漂移，以及可能由初始化失败派生的网络/sandbox 断言。
- Ruled out:
  - 编译期 Provider/TaskSpace TUI seam 缺失已补齐，不再是当前失败原因。
- Fix criteria:
  - fresh DB 与 0.149 Whale DB migration 矩阵通过；CLI Whale identity 测试通过；原 8 个失败 target 重跑无真实回归。
- Current conclusion: H-001、H-002、H-004、H-005 均已按证据最小修复；H-003 被反证。fresh/0.149 migration、Whale identity、TaskSpace debug、doctor、sandbox、完整 CLI 与 PTY 回归全部通过。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
- Resolution basis:
  - E-006：state migration 17/17 覆盖 fresh DB、0.147 和已发布 0.149 Whale DB 的无损升级与未知历史 fail-closed。
  - E-007：Whale identity、queue、exit message、TaskSpace debug、doctor 与 sandbox 定向测试全部通过。
  - E-008：完整 CLI lib/integration 回归与 PTY 回归无失败。
- Close reason:
  - 修复标准全部满足；未通过 fallback、放宽断言或改变产品默认值换取测试通过。

## Hypothesis H-001: 上游 0051 与 Whale TaskSpace 0051 发生 migration 版本碰撞
- Status: confirmed
- Parent: P-001
- Claim: 0.151 新增 `0051_thread_artifacts.sql`，与 Whale 已发布的 `0051_taskspace_canonical_store.sql` 同时嵌入同一 SQLx migrator，fresh DB 第二次写入 version 51 时触发唯一键冲突。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 失败统一发生在 fresh `state_5.sqlite`，且目录中出现两个 0051 文件。
- Falsifiable predictions:
  - If true: migration 清单包含两个 version 51，fresh DB 稳定报 `_sqlx_migrations.version` 唯一键冲突。
  - If false: SQLx 不会同时嵌入两份 0051，或 fresh DB 错误来自已有数据污染。
- Diagnostic evidence plan:
  - Prediction or clause under test: 同一 embedded migrator 是否包含两个 version 51 并在 fresh DB 重现。
  - Signal: migration 文件清单与 fresh CLI/state 测试错误。
  - Capture method: 只读列举 migration 文件并运行 CLI 测试。
  - Event name or marker:
    - `_sqlx_migrations.version`
  - Correlation keys:
    - `state_5.sqlite`
  - Differentiates from:
    - 临时数据库污染或并发测试互相覆盖。
  - Supports if:
    - 两份 0051 共存且多个独立 fresh temp DB 均报同一唯一键错误。
  - Refutes if:
    - migration version 唯一或错误只出现在复用数据库。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: 0.151 上游 migration 占用 0051，Whale TaskSpace migration 必须无损后移并提供精确 checksum 兼容桥。
- Repair design readiness: ready
- Next step: none；修复已由 E-006 验证。
- Blocker:
  - none
- Close reason:
  - 根因已修复并覆盖 fresh 与历史 DB。

## Hypothesis H-002: 0.151 新增测试未适配 Whale 二进制身份
- Status: confirmed
- Parent: P-001
- Claim: exit-message 与 queue 测试仍硬编码 `codex`，而本项目唯一 CLI binary 是 `whale`，因此测试在产品行为正确时仍失败。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 失败文本直接显示实际输出为 `whale resume`、期望为 `codex resume`，queue 用例只查找 `CARGO_BIN_EXE_codex`。
- Falsifiable predictions:
  - If true: 生产 manifest binary 名为 `whale`，测试源码包含 `codex` 硬编码；改为 Whale test helper/期望后通过。
  - If false: `codex` binary 仍是受支持产品入口，或失败来自命令实现。
- Diagnostic evidence plan:
  - Prediction or clause under test: 测试是否绕过仓库既有 Whale binary helper 并硬编码上游身份。
  - Signal: Cargo binary 声明、测试 helper 调用和断言文本。
  - Capture method: 只读源码与失败日志比对。
  - Event name or marker:
    - `CARGO_BIN_EXE_codex`
  - Correlation keys:
    - `queue`
    - `format_exit_messages`
  - Differentiates from:
    - CLI 生产身份回退为 Codex。
  - Supports if:
    - manifest 仅构建 `whale` 且测试硬编码 `codex`。
  - Refutes if:
    - manifest 同时承诺 `codex` binary。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: 这是上游测试 overlay 漏适配，不应新增 Codex 产品入口。
- Repair design readiness: ready
- Next step: none；修复已由 E-007/E-008 验证。
- Blocker:
  - none
- Close reason:
  - 测试与产品 binary identity 已统一为 Whale。

## Hypothesis H-003: 剩余网络与 sandbox 失败是 state 初始化失败的派生症状
- Status: refuted
- Parent: P-001
- Claim: `doctor_enterprise_network` 未收到请求和 `sandbox_cloud_config` 未执行 Whale version，均因命令在 state migration 初始化阶段提前退出，而非 0.151 网络或 sandbox 行为回归。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - 同一测试批次中 app-server/delete 明确在 state 初始化处提前退出；这两个失败也表现为预期动作从未发生。
- Falsifiable predictions:
  - If true: 修复 migration 后原样重跑会达到 wiremock 请求和 sandboxed Whale 命令。
  - If false: migration 修复后仍以相同断言失败，并暴露独立调用链问题。
- Diagnostic evidence plan:
  - Prediction or clause under test: 消除 H-001 后失败是否自然消失。
  - Signal: 两个定向测试的退出码、stderr 与 wiremock/命令命中。
  - Capture method: migration 修复后原样定向重跑，不增加生产 fallback。
  - Event name or marker:
    - `invalid_custom_ca_falls_back_to_system_roots`
    - `sandbox_fetches_and_enforces_cloud_managed_permission_profile`
  - Correlation keys:
    - test name
  - Differentiates from:
    - 独立网络代理或 permission-profile 回归。
  - Supports if:
    - 两项在 migration 修复后通过。
  - Refutes if:
    - 仍失败且命令已越过 state 初始化。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: migration 修复后两项仍失败，必须按独立身份/命令路径继续诊断。
- Repair design readiness: not applicable
- Next step: none；该假设已被反证，独立根因由 H-005 接管并完成修复。
- Blocker:
  - none
- Close reason:
  - 证据与必要预测矛盾。

## Hypothesis H-004: 0.151 机械替换删除了既有 TaskSpace debug export seam
- Status: confirmed
- Parent: P-001
- Claim: `DebugSubcommand::TaskspaceMap`、参数结构、dispatch arm 和 exporter 函数从 CLI 主文件被整段删除，导致既有 `whale debug taskspace-map` 入口不可用。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 测试返回 `unrecognized subcommand 'taskspace-map'`，staged diff 显示四个既有代码块被删除。
- Falsifiable predictions:
  - If true: HEAD 同时存在 enum/args/dispatch/function，当前树四者均缺失；最小回放后定向测试通过。
  - If false: 当前 parser 仍注册该命令，失败来自 state/runtime。
- Diagnostic evidence plan:
  - Prediction or clause under test: parser 注册链是否完整丢失。
  - Signal: HEAD/current 源码差分与 parser 错误。
  - Capture method: 只读源码比对和定向测试。
  - Event name or marker:
    - `taskspace-map`
  - Correlation keys:
    - `DebugSubcommand`
  - Differentiates from:
    - TaskSpace state migration或导出逻辑回归。
  - Supports if:
    - parser 在进入 exporter 前即拒绝命令且四个 seam 均缺失。
  - Refutes if:
    - exporter 已被调用。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: 既有隐藏 debug seam 被 0.151 replay 漏删，按 HEAD 最小恢复。
- Repair design readiness: ready
- Next step: none；修复已由 E-007/E-008 验证。
- Blocker:
  - none
- Close reason:
  - 既有隐藏 debug seam 已最小恢复且 3/3 通过。

## Hypothesis H-005: 0.151 主 CLI clap 身份回退为 Codex
- Status: confirmed
- Parent: P-001
- Claim: 0.151 机械替换把主 parser 的名称、bin_name 和 usage 从 Whale 改回 Codex，使实际 `whale --version` 输出 `codex-cli 0.0.6`，并导致 sandbox 身份断言失败。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 实际 binary 文件名为 whale，但运行时版本前缀为 codex-cli；current/HEAD 差分直接显示 clap metadata 回退。
- Falsifiable predictions:
  - If true: 恢复 HEAD 的 clap name/bin_name/usage 后 `whale --version` 以 `whale` 开头，sandbox 用例越过身份断言。
  - If false: 版本前缀由别处固定，恢复 parser metadata 不改变输出。
- Diagnostic evidence plan:
  - Prediction or clause under test: `CommandFactory` metadata 是否控制版本输出前缀。
  - Signal: `whale --version` 输出与 clap 声明。
  - Capture method: 运行 binary 并比对 HEAD/current 源码。
  - Event name or marker:
    - `whale --version`
  - Correlation keys:
    - `MultitoolCli`
  - Differentiates from:
    - sandbox 未执行子命令。
  - Supports if:
    - 当前输出 `codex-cli` 且 parser 声明 `bin_name = "codex"`。
  - Refutes if:
    - 当前 parser 已声明 Whale。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-005
- Conclusion: 这是 W3 identity seam 漏回放，不是新产品决策。
- Repair design readiness: ready
- Next step: none；修复已由 E-007/E-008 验证。
- Blocker:
  - none
- Close reason:
  - clap identity、completion、help 与 sandbox 调用链均已恢复并验证。

## Evidence E-001: 独立 fresh DB 与 migration 清单确认重复 0051
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `/tmp/codex-0151-cli-tests.log`；`state/migrations` 文件清单
- Prediction or plan link:
  - H-001：两份 version 51 共存并在 fresh DB 重现。
- Matched signal:
  - `0051_thread_artifacts.sql` 与 `0051_taskspace_canonical_store.sql` 共存；多个不同 `/tmp/.tmp*/state_5.sqlite` 均报 `(code: 1555) UNIQUE constraint failed: _sqlx_migrations.version`。
- Correlation keys:
  - `state_5.sqlite`
- Raw content:
  ```text
  0051_taskspace_canonical_store.sql
  0051_thread_artifacts.sql
  failed to migrate state DB ... UNIQUE constraint failed: _sqlx_migrations.version
  ```
- Interpretation: 错误不依赖旧 DB 或单个测试，根因是 embedded migration version 重复。
- Time: 2026-09-01 02:15

## Evidence E-002: 测试期望与 Whale binary 身份直接冲突
- Related hypotheses:
  - H-002
- Direction: supports
- Type: test
- Source: `/tmp/codex-0151-cli-tests.log`
- Prediction or plan link:
  - H-002：新增测试硬编码上游身份。
- Matched signal:
  - exit-message 实际为 `whale resume`、期望为 `codex resume`；queue 六项均报告只查找 `CARGO_BIN_EXE_codex`。
- Correlation keys:
  - `format_exit_messages`
  - `queue`
- Raw content:
  ```text
  < To continue this session, run whale resume ...
  > To continue this session, run codex resume ...
  could not locate binary "codex"; tried env vars ["CARGO_BIN_EXE_codex"]
  ```
- Interpretation: 生产 Whale identity 正确，测试 overlay 漏适配。
- Time: 2026-09-01 02:15

## Evidence E-003: 两项未命中预期动作但尚未暴露独立根因
- Related hypotheses:
  - H-003
- Direction: neutral
- Type: test
- Source: `/tmp/codex-0151-cli-tests.log`
- Prediction or plan link:
  - H-003：需要在 H-001 修复后差分重跑。
- Matched signal:
  - wiremock 收到 0 次请求；sandbox 测试称预期 Whale version 命令未运行。
- Correlation keys:
  - `invalid_custom_ca_falls_back_to_system_roots`
  - `sandbox_fetches_and_enforces_cloud_managed_permission_profile`
- Raw content:
  ```text
  The server did not receive any request.
  expected the sandboxed Whale version command to run
  ```
- Interpretation: 能证明调用链提前结束，但当前证据不能区分 migration 派生失败与独立 0.151 回归。
- Time: 2026-09-01 02:15

## Evidence E-004: parser 在 TaskSpace exporter 之前拒绝既有命令
- Related hypotheses:
  - H-004
- Direction: supports
- Type: code-location
- Source: `cli/src/main.rs` HEAD/current diff；`debug_taskspace_map` 定向重跑
- Prediction or plan link:
  - H-004：enum/args/dispatch/function 四段同时缺失。
- Matched signal:
  - staged diff 删除四段实现；运行返回 `error: unrecognized subcommand 'taskspace-map'`。
- Correlation keys:
  - `DebugSubcommand::TaskspaceMap`
- Raw content:
  ```text
  error: unrecognized subcommand 'taskspace-map'
  ```
- Interpretation: 尚未进入 state DB/exporter，根因是 CLI seam 删除。
- Time: 2026-09-01 02:20

## Evidence E-005: 实际 Whale binary 报告 Codex CLI 身份
- Related hypotheses:
  - H-005
- Direction: supports
- Type: reproduction
- Source: `target/debug/whale --version`；`cli/src/main.rs` HEAD/current diff
- Prediction or plan link:
  - H-005：parser metadata 回退控制版本前缀。
- Matched signal:
  - 实际输出 `codex-cli 0.0.6`；当前声明 `bin_name = "codex"`，HEAD 声明 name/bin_name/usage 均为 Whale。
- Correlation keys:
  - `MultitoolCli`
- Raw content:
  ```text
  codex-cli 0.0.6
  ```
- Interpretation: sandbox 确实执行了 binary，但其用户可见身份错误。
- Time: 2026-09-01 02:20

## Evidence E-006: migration 兼容矩阵验证修复
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test
- Source: `cargo test -p codex-state 'migrations::tests' --no-fail-fast`
- Prediction or plan link:
  - H-001：版本后移和精确历史修复后，fresh 与已发布历史均可升级，未知历史拒绝改写。
- Matched signal:
  - 17/17 migration tests 通过，包括 fresh schema、0.147、0.149 Whale 51/52 历史升级和未知 checksum fail-closed。
- Correlation keys:
  - `repair_whale_0149_taskspace_migration_versions`
  - `0052_taskspace_canonical_store.sql`
  - `0053_taskspace_relational_store.sql`
- Raw content:
  ```text
  test result: ok. 17 passed; 0 failed
  ```
- Interpretation: 修复既避免新库重复 version，也不把未知数据库历史静默改写为已知迁移。
- Time: 2026-09-01 02:38

## Evidence E-007: 原失败路径定向回归全部通过
- Related hypotheses:
  - H-002
  - H-004
  - H-005
  - H-003
- Direction: supports
- Type: test
- Source: CLI 定向测试日志与 `/tmp/codex-0151-cli-bin-final.log`
- Prediction or plan link:
  - 修复 Whale test identity、TaskSpace parser seam 和 clap identity 后，各独立失败路径应恢复。
- Matched signal:
  - CLI 主二进制 256/256；queue 6/6；TaskSpace debug 3/3；exit message 9/9；doctor enterprise network 1/1；sandbox cloud config 1/1。
- Correlation keys:
  - `whale`
  - `taskspace-map`
  - `invalid_custom_ca_falls_back_to_system_roots`
  - `sandbox_fetches_and_enforces_cloud_managed_permission_profile`
- Raw content:
  ```text
  test result: ok. 256 passed; 0 failed
  ```
- Interpretation: 网络与 sandbox 并非仅由 migration 阻断；它们在 Whale CLI 身份及测试调用路径恢复后通过。
- Time: 2026-09-01 02:42

## Evidence E-008: 完整 CLI 与 PTY 回归无失败
- Related hypotheses:
  - H-001
  - H-002
  - H-004
  - H-005
- Direction: supports
- Type: test
- Source: `/tmp/codex-0151-cli-tests-final2.log`；`/tmp/codex-0151-pty-final.log`
- Prediction or plan link:
  - P-001 fix criteria：原 8 个失败 target 消失且通用 CLI/PTY substrate 无新增回归。
- Matched signal:
  - `cargo test -p codex-cli --lib --tests --no-fail-fast` 所有 lib、bin 和 integration targets 通过；`codex-utils-pty` 27/27 通过。
- Correlation keys:
  - `codex-cli`
  - `codex-utils-pty`
- Raw content:
  ```text
  test result: ok. 13 passed; 0 failed
  test result: ok. 256 passed; 0 failed
  test result: ok. 27 passed; 0 failed
  ```
- Interpretation: 诊断修复满足局部与组合回归门槛，可关闭 P-001。
- Time: 2026-09-01 02:44
