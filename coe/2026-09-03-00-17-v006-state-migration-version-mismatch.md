# Problem P-001: v0.0.6 无法打开被 0.151 开发版升级的 state DB
- Status: open
- Created: 2026-09-03 00:17
- Updated: 2026-09-03 00:34
- Objective: 在不丢失本机 Whale 会话数据的前提下，恢复 release v0.0.6 启动，并阻止同类开发环境污染再次发生。
- Symptoms:
  - `whale --yolo` 报 migration 51 已应用但内容被修改，启动中止。
- Expected behavior:
  - 较旧的已发布 Whale 能安全识别较新开发版 state DB；开发验证不污染 release 使用的 `~/.whale`。
- Actual behavior:
  - v0.0.6 把数据库中 0.151 的 migration 51 视为自身同号 TaskSpace migration 被篡改。
- Impact:
  - 本机全局安装的 Whale v0.0.6 完全无法进入交互界面。
- Reproduction:
  - 在当前 `/home/zhangxu/.whale/state_5.sqlite` 上运行 `whale --yolo`。
- Environment:
  - Linux；全局 npm package `@ceasarxuu/whalecode@0.0.6`；数据库已包含 0.151 schema versions 51-53；开发分支 `whalecode-alpha` 基于 Codex 0.151。
- Known facts:
  - 数据库 `PRAGMA integrity_check` 返回 `ok`。
  - 数据库 migration 51 是 `thread artifacts`，checksum 与当前 0.151 migration 文件精确一致；52/53 是 TaskSpace migrations。
  - release v0.0.6 tag 早于 0.151 substrate cutover。
- Ruled out:
  - SQLite 文件物理损坏。
- Fix criteria:
  - 合成的“0.151 DB → v0.0.6 运行”复现先稳定失败；修复后保留原数据库与关键表/记录并可启动；fresh 与既有迁移测试无回归；本机 release 启动验证使用可恢复备份且不丢数据。
- Current conclusion: H-001/H-002 已确认。当前开发版已修正错误分类并验证可打开 0.151 schema；全局 v0.0.6 仍是不可前向兼容的已发布旧二进制。恢复全局命令必须在“升级二进制”或“备份并重建 release state DB”之间选择，未经用户明确授权不执行后者。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: 0.151 与 v0.0.6 对 migration 51 的不同占用触发 SQLx VersionMismatch
- Status: confirmed
- Parent: P-001
- Claim: 开发版先把全局 state DB 升级到 0.151（51=thread artifacts，52/53=TaskSpace），随后 v0.0.6 仍嵌入 51/52=TaskSpace；SQLx 在校验已知 version 51 时发现 checksum 不同并中止，错误被误分类为数据库损坏。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - 用户错误明确是 migration 51 modified；数据库 migration 元数据与当前 0.151 文件逐字校验一致。
- Falsifiable predictions:
  - If true: v0.0.6 tag 的 migration 51 checksum 与数据库不同且内容为 TaskSpace；用 v0.0.6 打开 0.151 合成 DB 可稳定复现同一错误。
  - If false: v0.0.6 的 migration 51 与数据库 checksum 相同，或隔离复现不失败。
- Diagnostic evidence plan:
  - Prediction or clause under test: v0.0.6 嵌入 migration 51 与 0.151 数据库的同号异义。
  - Signal: tag 文件 checksum、数据库 `_sqlx_migrations` 行、隔离启动错误。
  - Capture method: 只读 git/sqlite 检查，并在临时 Whale home 复制合成数据库后运行对应二进制。
  - Event name or marker:
    - `migration 51 was previously applied but has been modified`
  - Correlation keys:
    - migration version 51
  - Differentiates from:
    - SQLite 物理损坏、并发锁、随机文件污染。
  - Supports if:
    - tag/current 对 version 51 的 checksum 不同，且隔离 DB 稳定产生相同 VersionMismatch。
  - Refutes if:
    - checksum 相同或隔离运行越过 migration。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-005
  - E-006
- Conclusion: v0.0.6 的 51/52 是 TaskSpace，而 0.151 的 51 是 thread artifacts、TaskSpace 后移至 52/53；旧 binary 对已知 51 执行 checksum 校验，`ignore_missing` 无法解决同号异义。
- Repair design readiness: ready
- Next step: 修正 migration 不兼容的用户提示，并验证当前开发版可读取 0.151 schema。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: 开发验证绕过 workspace home 隔离并升级了 release 数据库
- Status: confirmed
- Parent: P-001
- Claim: 某次 0.151 开发版运行直接使用 `/home/zhangxu/.whale`，而不是 workspace bootstrap 生成的隔离 home，因此把 release 数据库升级到 51-53。
- Layer: environment
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - release DB 中存在当前开发树独有的 0.151 migration 51-53。
- Falsifiable predictions:
  - If true: workspace 运行合同提供隔离 home，数据库或 shell/history 证据显示开发二进制曾以全局 home 运行。
  - If false: v0.0.6 本身已发布 0.151 migration 51-53，或开发路径没有隔离能力。
- Diagnostic evidence plan:
  - Prediction or clause under test: 0.151 schema 是否只能由开发版写入全局 release home。
  - Signal: v0.0.6 tag schema、workspace context 状态和安装/运行脚本环境合同。
  - Capture method: 只读检查 tag、workspace safety 输出和脚本。
  - Event name or marker:
    - `WHALE_HOME`
  - Correlation keys:
    - `/home/zhangxu/.whale/state_5.sqlite`
  - Differentiates from:
    - release v0.0.6 自身正常升级。
  - Supports if:
    - v0.0.6 不含该 schema 且 workspace 明确提供隔离 home。
  - Refutes if:
    - release 本身包含 0.151 schema。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-002
  - E-004
- Conclusion: workspace safety 明确把开发 runtime home 隔离到 `.local/state/whalecode/workspaces/.../home` 并承诺不触碰 `~/.whale`；全局 release DB 出现 9 月 1 日后才进入代码库的 0.151 schema，说明曾有开发运行绕过该合同。
- Repair design readiness: ready
- Next step: 所有后续开发验证只通过 `workspace_context.py exec --` 或显式临时 home。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 本机 SQLite 完整且 migration 51 与当前 0.151 文件一致
- Related hypotheses:
  - H-001
- Direction: supports
- Type: probe
- Source: `sqlite3 file:/home/zhangxu/.whale/state_5.sqlite?mode=ro`；`sha384sum state/migrations/0051_thread_artifacts.sql`
- Prediction or plan link:
  - H-001：数据库不是物理损坏，version 51 保存的是 0.151 thread artifacts。
- Matched signal:
  - integrity `ok`；数据库与文件 checksum 均为 `23360a...f29505`。
- Correlation keys:
  - migration version 51
- Raw content:
  ```text
  ok
  51|thread artifacts|1|23360A03A7FC307C3FD5BB8B432B66034DD8F8695CDBA698B45278C20DD712C1AF476B884E192237D80BAA08A5F29505
  ```
- Interpretation: “数据库损坏”提示不准确；当前错误属于合法 schema 历史与旧二进制不兼容。
- Time: 2026-09-03 00:17

## Evidence E-002: 全局二进制是 npm v0.0.6，而数据库已达到当前 0.151 schema
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: environment
- Source: `whale --version`、全局 npm package.json、数据库 migration 表、当前 migration 文件列表
- Prediction or plan link:
  - H-001/H-002：运行者与数据库 schema 代际不同。
- Matched signal:
  - binary/package 均为 0.0.6；DB 已有 51 thread artifacts、52/53 TaskSpace，checksum 全部匹配当前树。
- Correlation keys:
  - `/home/zhangxu/.local/bin/whale`
  - `/home/zhangxu/.whale/state_5.sqlite`
- Raw content:
  ```text
  whale 0.0.6
  51|thread artifacts
  52|taskspace canonical store
  53|taskspace relational store
  ```
- Interpretation: 故障发生在旧 release binary 读取较新开发 schema 的版本交叉路径。
- Time: 2026-09-03 00:17

## Evidence E-003: v0.0.6 对合成 0.151 migration ledger 稳定复现同一错误
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `/tmp/whale-v006-fresh-init.rIDUBl/mismatch.log`
- Prediction or plan link:
  - H-001：完整性正常的隔离数据库只需把 migration ledger 置为 0.151 布局，v0.0.6 就产生相同 VersionMismatch。
- Matched signal:
  - integrity `ok`；release binary 报 `migration 51 was previously applied but has been modified`。
- Correlation keys:
  - migration version 51
- Raw content:
  ```text
  ok
  failed to migrate state DB ... migration 51 was previously applied but has been modified
  ```
- Interpretation: 同一症状不依赖用户数据、SQLite 损坏或并发锁；同号 migration checksum 差异足以导致故障。
- Time: 2026-09-03 00:21

## Evidence E-004: release tag 与 workspace 隔离合同确认 schema 代际来源
- Related hypotheses:
  - H-002
- Direction: supports
- Type: environment
- Source: `git show v0.0.6`、`git show 6c2203c5c`、`workspace_context.py bootstrap plan --json`
- Prediction or plan link:
  - H-002：0.151 schema 晚于 release，正常 workspace 运行应写隔离 home 而非 `~/.whale`。
- Matched signal:
  - v0.0.6 tag 时间为 2026-08-28；0.151 cutover 为 2026-09-01；workspace runtime home 为 `/home/zhangxu/.local/state/whalecode/workspaces/whalecode-alpha-48d2219088/home`，`~/.whale` 被列为 untouched。
- Correlation keys:
  - `whalecode-alpha-48d2219088`
- Raw content:
  ```text
  v0.0.6: 78c46f4f 2026-08-28
  0.151 cutover: 6c2203c5 2026-09-01
  untouched: /home/zhangxu/.whale
  ```
- Interpretation: release 自身不可能生成 thread-artifacts migration 51；污染来源是绕过隔离的较新开发运行。
- Time: 2026-09-03 00:19

## Evidence E-005: 当前开发版可打开 0.151 migration schema
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `/tmp/whale-dev-migration-compat-smoke/compatible.log`；`whale debug taskspace-map`
- Prediction or plan link:
  - P-001：升级后的 binary 应越过 51/52/53 migration 校验且不进入模型路径。
- Matched signal:
  - 命令完成 state 初始化，只返回目标 thread 没有 TaskSpace binding 的业务错误；无 migration mismatch、无 provider 请求。
- Correlation keys:
  - `synthetic-current-schema`
- Raw content:
  ```text
  Error: thread 00000000-0000-0000-0000-000000000000 has no TaskSpace map binding
  ```
- Interpretation: 当前开发版与用户 DB 的 migration 代际一致，升级 binary 是无损恢复方向。
- Time: 2026-09-03 00:31

## Evidence E-006: migration mismatch 被明确归类为 build incompatibility
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `just test -p codex-cli state_db_recovery`；`/tmp/whale-dev-migration-compat-smoke/incompatible-pty.log`
- Prediction or plan link:
  - P-001：未知 migration checksum 不得再宣称数据库损坏，也不得自动备份/重建或进入模型路径。
- Matched signal:
  - 4/4 定向测试通过；PTY 启动输出 incompatible build、未删除/重建、升级/隔离指引；0 provider request markers。
- Correlation keys:
  - migration version 51
- Raw content:
  ```text
  Whale couldn't start because this local database was created by an incompatible Whale build.
  Whale did not delete or rebuild the database.
  Update Whale, or run the build that last used this database.
  Summary: 4 tests run: 4 passed
  ```
- Interpretation: 产品不再把兼容性拒绝误报为数据损坏，且保留 fail-closed 行为。
- Time: 2026-09-03 00:33

## Evidence E-007: state migration 兼容矩阵无回归
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test
- Source: `just test -p codex-state migrations::tests`
- Prediction or plan link:
  - P-001：提示修复不得改变 migration 执行、历史 repair 或未知 checksum fail-closed 行为。
- Matched signal:
  - 17/17 migration tests 通过，包括 fresh、0.147、0.149、0.151 与未知历史拒绝。
- Correlation keys:
  - `codex-state migrations::tests`
- Raw content:
  ```text
  Summary: 17 tests run: 17 passed
  ```
- Interpretation: 本次改动只影响用户提示，未改变数据库兼容性与保护边界。
- Time: 2026-09-03 00:30
