# PRD：Whale Dev 与 Release 完全隔离

- Status: Ready for implementation
- Created: 2026-09-03
- Updated: 2026-09-03
- Owner / requester: 项目维护者
- Source request: 后续开发版和 release 必须完全隔离；release 使用全局 `whale`，开发版使用全局 `whale-dev`，且多个 worktree 彼此隔离。
- Product Authority: Confirmed Product Decisions section

## Requester Review Summary

- Key decisions: `whale` 仅代表 release；`whale-dev` 仅代表开发版；开发运行按 worktree 选择独立 binary 和独立可写状态。
- Important exceptions: 当前实现沿用仓库既有 Linux workspace-safety 合同；Windows parity 保持既有 deferred 状态，不在本次无实机验证范围内宣称完成。
- Must-confirm before implementation: 无。
- Status reason: 命令身份、隔离粒度、仓库外行为和禁止回退规则均已由请求者直接确认。

## 1. Background And Product Intent

开发版曾直接使用 release 的 `~/.whale/state_5.sqlite`，将数据库升级到 Codex 0.151 migration 布局，导致已发布 v0.0.6 无法重新打开。仅依赖开发者记忆设置 `WHALE_HOME` 不足以防止再次污染，必须由安装入口和全局命令身份强制隔离。

## 2. Goals And Success Criteria

- `whale` 始终启动正式 release 安装，不解析或进入开发 workspace slot。
- `whale-dev` 始终从当前目录解析已 bootstrap 的 worktree，并启动该 worktree 的受证明开发 binary。
- 每个 worktree 使用不同的 runtime home 和 SQLite 路径；任何开发命令不得读写 `~/.whale`。
- 未就绪、未安装、仓库外或身份不匹配时 fail-closed，并给出机械恢复命令。
- 安装、运行、doctor 和自动化测试可以证明 release、两个开发 worktree 三方互不覆盖。

## 3. Users And Usage Context

- 主要用户：在 Linux 主机同时维护 release 使用环境和一个或多个 WhaleCode worktree 的开发者。
- release 使用：在任意目录执行 `whale`。
- 开发使用：在目标 worktree 或其子目录执行 `whale-dev`。

## 4. Scope

### In Scope

- Linux 全局 `whale-dev` 分发入口。
- workspace 安装产物的开发命令身份和 attestation。
- 基于当前目录的 worktree 解析、ready 校验、binary 校验和环境隔离。
- 安装器、bootstrap/doctor、测试与 runbook 更新。

### Out Of Scope

- 改写或迁移现有 `~/.whale` release 数据。
- 把开发 binary 发布到 npm release 包。
- 未经 Windows 实机或等价验证宣称 PowerShell parity 已完成。
- 多 worktree 之间自动同步 config、auth、sessions 或 plugins。

## 5. Core User Journey

1. 开发者在 worktree 执行 bootstrap plan/apply。
2. 开发者通过 workspace scope 安装当前构建。
3. 安装器把 binary 放入该 worktree 的隔离 slot，并确保全局 `whale-dev` dispatcher 可用。
4. 开发者在该 worktree 任意子目录执行 `whale-dev ...`。
5. dispatcher 根据 Git/worktree 上下文定位 slot，校验 ready 与 binary attestation，然后以该 worktree 的 `WHALE_HOME`/`CODEX_SQLITE_HOME` 启动。
6. `whale ...` 继续使用 release 安装和 `~/.whale`，不受开发安装影响。

## 6. Interaction And Information Design

- 用户可见命令只有两个稳定身份：`whale`（release）与 `whale-dev`（当前 worktree 的开发版）。
- `whale-dev --version` 应显示开发命令身份，并能追溯所选 workspace/binary；不得伪装成 release 入口。
- 失败消息必须说明具体原因和恢复动作，例如先 bootstrap 或安装 workspace binary。

## 7. Product Rules And State Logic

- `whale-dev` 的 workspace 选择只来自当前目录所在 Git worktree，不允许“最近安装版本”或全局活动槽位。
- 每个 canonical worktree root 派生稳定 workspace id，并映射到独立 state/data/binary roots。
- dispatcher 必须复用 workspace-safety 的 ready、identity 和 attestation 校验，不维护第二套弱化规则。
- 子进程必须覆盖 `WHALE_HOME`、`CODEX_SQLITE_HOME` 和 `PATH`，并移除 `CODEX_HOME`。
- persisted config/auth/history/sessions/logs/databases/plugins/skills/tmp 不跨 release 或 worktree 自动复制。
- 父进程环境中的 provider API key 可按既有 shell 环境继承；它不是由 Whale 持久化的共享状态。

## 8. Edge Cases, Errors, And Recovery

- 仓库外执行：拒绝运行，提示进入已 bootstrap worktree。
- worktree 未 bootstrap 或 marker stale：拒绝运行，输出对应 `bootstrap plan`/`apply` 恢复入口。
- slot 缺 binary、attestation 不匹配或 binary 过期：拒绝运行，提示 workspace scope 安装。
- 当前目录属于普通 Git 仓库但不是 WhaleCode workspace：拒绝运行，不搜索 PATH 上的 `whale`。
- release 或 legacy home 已存在：保持 untouched。
- 同时运行两个 worktree：binary、SQLite/WAL/log/tmp 路径均不同。

## 9. Content And Terminology

- `whale`: release command。
- `whale-dev`: worktree-aware development command。
- workspace slot: 单一 worktree 的隔离 state/data/binary 集合。
- 禁止把 workspace binary 称为 release，或把全局 dispatcher 称为实际 agent binary。

## 10. Acceptance Criteria

- Given 已安装 release，当任意 worktree 安装开发版后执行 `whale --version`，then release binary 路径、hash 和 `~/.whale` 均不改变。
- Given 两个已 bootstrap worktree，当分别执行 workspace install，then 两个 slot 各自保存不同 binary/attestation，互不覆盖。
- Given 位于 worktree A 子目录，当执行 `whale-dev --version`，then 只启动 A 的 binary，并设置 A 的 runtime home。
- Given 位于 worktree B，当执行同一命令，then 只启动 B 的 binary和 runtime home。
- Given 位于仓库外或 stale workspace，当执行 `whale-dev`，then 非零退出、无 fallback、无 `~/.whale` 写入。
- Given 父环境预设 `WHALE_HOME`、`CODEX_SQLITE_HOME` 或 `CODEX_HOME`，when 执行 `whale-dev`，then 前两者被目标 workspace 覆盖，后者被移除。
- Given release home 和两个 workspace home 已存在，when 并行执行零模型状态探针，then 三者 SQLite/WAL/log inode 与路径无交叉。
- 自动化验证不得提交自然语言 prompt 或产生 provider inference 请求。

## 11. Review Checklist And Sign-off Questions

- [x] release 与 dev 命令身份已确认。
- [x] dev 按 worktree 隔离已确认。
- [x] 仓库外 fail-closed 且禁止 fallback 已确认。
- [x] 不迁移或重建现有 release 数据。

## Confirmed Product Decisions

> PROTECTED USER-AUTHORITY SECTION
> Rows in this section MUST NOT be created, modified, deleted, reinterpreted,
> or superseded without explicit user approval for that specific decision change.
> Agent self-approval is forbidden.

| ID | Confirmed Decision | Must Do | Must Not Do | Rationale | Violation Signal | Confirmation | Status |
|---|---|---|---|---|---|---|---|
| PD1 | release 使用全局 `whale`，开发版使用全局 `whale-dev`。 | 安装和运行入口保持两个稳定身份。 | 开发安装不得覆盖或代理 `whale`。 | 防止开发构建污染正式使用环境。 | workspace 安装后 `whale` 路径/hash 变化，或 `whale-dev` 启动 release。 | user-confirmed-direct: “开发版采用 whale-dev 全局命令，release 采用 whale 全局命令” | active |
| PD2 | dev 与 release 的开发、安装和运行完全隔离。 | binary 与所有可写运行状态使用不同根路径；release home 保持 untouched。 | 不共享 SQLite、sessions、logs、config、auth、plugins、skills 或 tmp；不 fallback。 | 已发生 migration 代际污染并阻断 v0.0.6 启动。 | 开发进程访问 `~/.whale`，或 release 进程进入 workspace slot。 | user-confirmed-direct: “后续将dev 版本和release完全隔离开发、安装和运行” | active |
| PD3 | 多个 worktree 彼此隔离；全局 `whale-dev` 按当前目录选择 worktree。 | 从 cwd 解析 worktree slot；仓库外/未就绪时 fail-closed。 | 不使用最后安装优先的单一全局 dev slot，不跨 worktree 复用 home/binary。 | 保证并行开发和不同提交构建互不污染。 | worktree A 中 `whale-dev` 启动 B 的 binary/home，或仓库外回退到任意 binary。 | user-confirmed-direct: 选择“多个worktree 隔离”，对应上一轮推荐方案（含 cwd 路由与仓库外 fail-closed）。 | active |

## 12. Open Questions And Risks

- Windows parity 仍受现有本机验证条件限制；不得将 Linux 完成状态外推为 Windows 已验证。
- 已打开的 shell 可能缓存旧命令位置；安装完成后需要以可检测方式提示 `hash -r` 或重新打开 shell。
- dispatcher 必须避免依赖当前 worktree 内尚未可信的可执行脚本来决定安全边界。
- 多个 worktree 共享的无状态 dispatcher 必须使用单调 revision；旧 worktree 安装不得降级全局 dispatcher，同 revision 内容分叉必须 fail-closed。

## 13. Implementation Notes

- 复用现有 workspace identity、marker、ready、attestation 和 runtime environment 合同。
- 全局 dispatcher 是从干净、已证明 worktree 安装的自包含安全入口；只负责定位和验证目标 worktree 后启动精确 binary，不持有开发状态或“当前活动版本”。
- dispatcher revision 是安装兼容协议；每次修改 dispatcher 行为必须提升 revision。
- 验证以 fake HOME/XDG 和临时 Git worktree 完成，不读取、复制或修改真实 `~/.whale`。
