# PRD：开发工作区 Bootstrap 生命周期门禁

- Status: Ready for implementation
- Created: 2026-08-06
- Updated: 2026-08-06
- Owner / requester: 项目维护者
- Source request: 为当前及未来 WhaleCode 分支提供统一、显式、可计划执行的本机工作区检查与隔离流程。

## Requester Review Summary

- Key decisions:
  - Bootstrap 是开发者和 Coding Agent 开始在某个 branch/workspace 上工作前必须显式执行的独立流程。
  - 流程采用 `plan → apply` 两阶段；plan 只读，apply 才创建或更新本地状态。
  - 项目统一入口执行轻量代码门禁；直接使用底层 Git/Cargo 命令不做重型拦截。
  - `README.md`、`AGENTS.md` 和开发运行手册必须声明这一前置要求。
  - workspace 目录是稳定身份；branch 是登记状态。切换 branch 后必须重新 bootstrap，但复用该 workspace 已隔离的目录。
  - apply 创建身份标记、隔离运行时目录、本地环境配置并执行 doctor；不迁移、不覆盖 legacy 数据。
- Important exceptions:
  - 只存在于 Git refs、尚未检出到本机 workspace 的 branch 不创建运行时资源。
  - 不拦截开发者绕过项目入口直接执行 `cargo`、`git` 等底层命令；规范约束是主要治理手段，代码门禁是基础兜底。
  - Windows 能力继续延期，不阻塞 Linux 首版。
- Must-confirm before implementation: 无产品阻塞项；技术路径和工作单元见配套工程计划。
- Status reason: 用户已确认触发、交互、门禁强度、apply 范围和 branch 切换策略。

## 1. 背景与产品意图

同一设备可能同时存在多个 WhaleCode clone、Git worktree、branch 和开发二进制。Git 能隔离源码与 index，却不会自动隔离 `WHALE_HOME`、SQLite、sessions、logs、开发安装槽和某些 runner 的默认二进制。开发者或 Coding Agent 如果直接开始构建、测试或运行，可能把一个 workspace 的结果错误归因到另一个 workspace。

本功能把工作区检查从一次性人工排障变成可重复的开发生命周期步骤：任何 branch 在某个本机 workspace 中开始工作前，都先获得一份只读计划，确认后登记身份、隔离资源并通过 doctor。目标是让后续新增 workspace 自动遵循同一合同，而不是为当前两个目录维护特例。

## 2. 目标与成功标准

### 目标

1. 开发者能在一个命令族内完成 workspace 检查、计划预览、隔离初始化和验证。
2. Coding Agent 能从 `AGENTS.md` 得知 bootstrap 是开始改代码前的必要动作，并能使用机械化状态判断是否完成。
3. 同机 workspace 不共享 Whale 可变运行时状态或开发二进制槽位。
4. branch 切换可被识别并要求重新确认，不把首次 bootstrap 永久视为有效。
5. 流程默认不迁移、不覆盖、不删除历史数据，不发起模型请求。

### 成功标准

- 新 workspace 在 apply 前，项目统一的构建、运行、安装和测试入口返回稳定的“未 bootstrap”诊断。
- `plan` 不改变 repo、Git 配置、用户 profile、legacy `~/.whale` 或 workspace 状态。
- `apply` 后 doctor 能证明 workspace identity、当前 branch、runtime home、SQLite home 和 binary slot 一致且不与其他已登记 workspace 冲突。
- 同一 workspace 切换 branch 后，统一入口拒绝继续，重新 apply 后恢复；隔离目录不重复创建。
- README、AGENTS 和开发手册对人类与 Agent 使用同一组术语和命令。

## 3. 用户与使用场景

### 用户

- 在本机创建或检出 WhaleCode branch 的开发者。
- 通过 Codex/Whale 等 Coding Agent 在仓库内执行任务的维护者。
- 维护构建、测试、benchmark、cache regression 和本地安装入口的工程人员。

### 典型场景

- 开发者通过 VS Code、Git CLI 或其他工具创建 branch/worktree，随后显式执行 bootstrap。
- Agent 进入一个新 workspace，先依据 `AGENTS.md` 检查 bootstrap 状态，再开始修改代码。
- 开发者在同一 worktree 切换 branch，doctor 发现登记 branch 不一致并要求重新执行流程。
- 多个 worktree 并行构建和运行，各自使用独立运行时 home 与二进制槽位。

## 4. 范围

### In Scope

- Linux 本机的 Git clone 与 linked worktree。
- `bootstrap plan`、`bootstrap apply`、`doctor` 和受控命令执行入口。
- workspace/branch 状态、隔离目录和非秘密本地环境配置。
- 项目统一构建、运行、安装、测试入口的轻量前置检查。
- 根级 README、AGENTS、开发 runbook 和 VS Code 使用说明。
- 当前 `whalecode-codex` 与 `whalecode-alpha` 作为首批兼容和并发验收样本。

### Out Of Scope

- 接管 branch/worktree 的创建；开发者仍可使用任意 Git 或 IDE 工具。
- 对直接执行的所有 `git`、`cargo`、`bazel` 命令做系统级强制拦截。
- 自动迁移、合并、删除或软链 `~/.whale` 中的 config、auth、history、sessions、plugins 或 skills。
- 为纯 Git ref 预创建 workspace 资源。
- 引入 daemon、全局数据库、容器或强制安装 direnv。
- Windows parity 和真实 Whale Agent 模型运行。

## 5. 核心用户旅程

1. 用户通过任意方式创建 branch，并把它检出到一个本机 workspace。
2. 用户或 Agent 执行 `workspace bootstrap plan`。
3. 系统只读检查 canonical repo root、Git common-dir、当前 branch、已有登记、运行时目录、binary slot、共享资源冲突和 legacy 状态，输出拟执行动作及警告。
4. 用户确认计划后，携带 plan 输出的 fingerprint 执行 `workspace bootstrap apply`。
5. 系统验证计划仍对应当前 workspace/branch，创建或更新身份标记、隔离目录和本地环境配置，然后自动执行 doctor。
6. doctor 成功后，项目统一入口允许构建、测试、运行和本地安装。
7. 若 worktree 切换 branch，登记状态失效；用户重新执行 plan/apply。系统复用 workspace 隔离目录，只更新 branch 绑定和验证证据。

## 6. 交互与信息设计

### Plan 输出

必须同时提供适合人类阅读的摘要和适合脚本/Agent 判断的结构化输出，至少包含：

- canonical workspace root、workspace id、当前 branch；
- Git common-dir 与 remote 的非敏感摘要；
- 将使用的 runtime home、SQLite home 和 binary slot；
- 当前是否已登记、是否发生 branch 切换、是否与其他 workspace 冲突；
- apply 将创建或更新的对象；
- 不会触碰的 legacy 路径和数据；
- 稳定的 plan fingerprint；plan 不落盘，fingerprint 由用户或调用入口显式传给 apply。

### Apply 输出

- 显示已创建、已复用和已更新的对象；
- 显示 doctor 的成功或失败状态与稳定诊断码；
- 不输出凭据、完整敏感环境变量或凭据化 remote URL；
- 失败时明确说明未执行、已执行到何处以及安全恢复动作。

## 7. 产品规则与状态逻辑

### 状态

| 状态 | 含义 | 统一入口行为 |
| --- | --- | --- |
| Unbootstrapped | workspace 没有有效登记 | 阻断并提示先执行 plan |
| Ready | workspace、branch 与 doctor 证据一致 | 允许继续 |
| Stale | branch、root、common-dir 或关键配置与登记不一致 | 阻断并要求重新 plan/apply |
| Conflict | identity 或资源已绑定到其他 workspace | 阻断；不得自动覆盖 |
| DoctorFailed | apply 已尝试但验证未通过 | 阻断并输出恢复建议 |

### 规则

- workspace identity 以 canonical repo root 为主，不以 branch 名作为唯一身份。
- workspace id 必须由目录可读名称和 canonical root 摘要自动生成，同名目录不要求人工改名。
- marker 必须记录当前 branch；普通 commit 前进不导致状态失效，branch 名变化导致失效。
- detached HEAD 默认不能成为 Ready，doctor 应给出明确诊断；只读 plan 仍可运行。
- apply 必须验证 plan 所依据的 workspace 和 branch 未变化，避免确认后作用于另一上下文。
- apply 必须幂等；对同一有效状态重复执行不创建第二套目录。
- branch 切换后的 apply 复用当前 workspace 的隔离目录，不自动清空历史状态。
- 统一入口只做快速基础检查；深度资源扫描由 plan/doctor 承担。
- 任何冲突均 fail closed，不提供静默 fallback 到 PATH 或 `~/.whale`。

## 8. 边界、错误与恢复

- **目录移动或重命名**：登记变为 Stale；重新 plan，不能静默接管旧 identity。
- **同名目录**：通过 canonical root 摘要生成不同 workspace id；任何摘要碰撞或已绑定不同 root 的情况仍 fail closed，不自动覆盖。
- **工作树已有未提交修改**：plan 显示警告但保持只读；apply 不修改源码或 index，因此可继续，最终 doctor 必须通过。
- **branch 在 plan/apply 间变化**：apply 拒绝使用过期计划，要求重新 plan。
- **遗留共享 home 存在**：只报告，不复制、不删除、不修改。
- **已有 Whale 进程仍在运行**：若该进程使用将发生冲突的目录，apply 停止；只读 plan 可以完成。
- **部分目录已创建**：apply 使用幂等检查继续或返回 DoctorFailed；恢复不得使用不可恢复删除。
- **凭据不可用**：bootstrap 仍可完成结构隔离，但需要凭据的真实运行保持不可用；不得复制 legacy auth 作为 fallback。

## 9. 内容与术语

- 对外统一使用 `workspace bootstrap`、`plan`、`apply`、`doctor`、`workspace identity`、`Ready` 和 `Stale`。
- 文档明确区分 branch、Git worktree 和 workspace：branch 是 Git ref；worktree 是 Git 检出机制；workspace 是实际开发目录及其隔离运行时资源。
- AGENTS 约束使用命令式表述：“开始修改、构建、测试或运行前，必须检查并完成 workspace bootstrap。”
- 错误消息只陈述机械状态、路径摘要和恢复命令，不伪装成 Agent 的自然语言回答。

## 10. 验收标准

- [ ] 全新 clone 和 linked worktree 均可执行 plan/apply 并进入 Ready。
- [ ] plan 的文件系统与 Git 状态前后无变化。
- [ ] apply 必须携带并验证 plan fingerprint；plan/apply 之间发生 branch、root 或 common-dir 变化时拒绝执行。
- [ ] apply 幂等，且不改变源码、index、Git refs、用户 profile 或 legacy `~/.whale`。
- [ ] 两个并行 workspace 的 runtime、SQLite、logs、sessions、tmp 和 binary slot 无路径交集。
- [ ] 同一 workspace 切换 branch 后为 Stale，重新 apply 后 Ready 且复用原隔离目录。
- [ ] detached HEAD、identity collision、错误 binary attestation 和共享 home 均有稳定诊断码。
- [ ] 项目统一入口在 Unbootstrapped/Stale/Conflict/DoctorFailed 时于副作用前退出。
- [ ] 直接底层命令不被系统级拦截，README/AGENTS 明确其责任边界。
- [ ] README、AGENTS、runbook、VS Code 说明和 CLI `--help` 术语一致。
- [ ] 所有自动验证均为无模型测试，不新增 Whale Agent run 账本记录。

## 11. Review Checklist And Sign-off Questions

- [x] Bootstrap 是独立流程，不接管 branch/worktree 创建。
- [x] 两阶段交互和确认边界已明确。
- [x] 轻量代码门禁与规范治理的责任边界已明确。
- [x] branch 切换后的失效与复用策略已明确。
- [x] legacy 数据、凭据和 Windows 非目标已明确。
- [ ] 实现完成后由请求者确认命令命名与实际开发体验。

## 12. Clarification Decision Log

| Topic | Decision | Rationale | Source Round |
| --- | --- | --- | --- |
| 触发方式 | branch/worktree 创建后显式执行独立 bootstrap | 兼容 VS Code、Git CLI 和其他工具，不接管 Git 工作流 | Round 1：1A |
| 执行方式 | plan → apply 两阶段 | 用户先看到检查结果和拟执行动作，再产生状态 | Round 1：2A |
| 门禁强度 | 统一入口轻量阻断，规范文档约束为主 | 面向 Agent Coding 提供可靠前置条件，同时避免重型系统拦截 | Round 2：3A + 用户补充 |
| Apply 范围 | 创建身份、隔离目录、环境配置并自动 doctor；不迁移旧数据 | 完成可用闭环，并保持历史数据安全可逆 | Round 2：4A |
| Branch 切换 | workspace 为主身份，branch 变化使状态失效；重新 apply 并复用目录 | 落实每个 branch 开工前检查，同时避免重复缓存和目录膨胀 | Round 3：5A |
