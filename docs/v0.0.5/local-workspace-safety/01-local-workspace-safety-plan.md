# 通用 Branch/Workspace Bootstrap 安全管理工程计划

- 文档状态：计划完成，待执行授权
- 计划模式：Plan Authoring
- 创建日期：2026-08-06
- 更新日期：2026-08-06
- 适用版本：WhaleCode v0.0.5
- 产品基线：`prd/2026-08-06-workspace-bootstrap.md`
- 首批验证对象：`$HOME/whalecode-codex`、`$HOME/whalecode-alpha`
- 范围：Linux 本机开发、构建、测试、CLI 安装和运行时状态隔离

## 1. 结论与目标合同

现有两个 workspace 没有共享 Git common-dir、index、HEAD 或默认 Cargo target；当前串扰来自 Git 之外的用户级可变资源：默认 `~/.whale`、SQLite、sessions、logs、plugins、skills、临时目录、PATH 上的全局 `whale` 和 runner 的默认 binary 解析。

本方案不是只修复两个已知目录，而是建立所有未来 WhaleCode workspace 共用的生命周期合同：

> branch 被检出到实际 workspace 后，开发者或 Coding Agent 必须先显式执行 `bootstrap plan → bootstrap apply`。workspace canonical root 是稳定身份，当前 branch 是可失效登记；branch 切换后重新 bootstrap，但复用 workspace 已隔离的运行时和二进制目录。

纯 Git ref 不触发资源创建。项目统一入口实施轻量 fail-closed 检查，README、AGENTS 和 runbook 承担主要流程治理；不拦截开发者直接调用 Git/Cargo 等底层工具。

## 2. 已知事实与设计依据

### 2.1 当前事实

- `whalecode-codex` 与 `whalecode-alpha` 属于两套 Git common-dir，源码编辑和提交不会彼此覆盖。
- 两边未显式设置时都会使用 `~/.whale`；不同代码版本因此可能共享状态库和 session。
- PATH 上的 `~/.whale/bin/whale` 来自 alpha 的旧安装，codex workspace 可能错误执行它。
- cache regression 当前存在从全局路径选择 binary 的入口。
- Cargo registry、Git download cache 和 Rustup 工具链属于可安全共享的只读/内容寻址资源；workspace target 与运行时状态不可共享。

### 2.2 外部依据

1. [Git worktree 官方文档](https://git-scm.com/docs/git-worktree.html)：linked worktree 共享 common repository，但 HEAD、index 和 worktree metadata 分离；branch 与 worktree 是不同对象。
2. [Cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html#buildtarget-dir)与[Cargo build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html)：默认 target 位于 workspace root；跨 workspace 编译复用应使用专门缓存，而非共享同一 target-dir。
3. [Bazel output directory layout](https://bazel.build/remote/output-directories)：默认 outputBase 由 canonical workspace path 派生，显式共享 output root 才会破坏隔离。
4. [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)：持久状态、数据、缓存和运行时对象应使用不同基础目录，用户状态目录需限制权限。
5. [SQLite WAL](https://sqlite.org/wal.html)：WAL 改善并发，但不提供不同代码版本之间的 schema 与状态语义隔离。
6. [direnv](https://direnv.net/)：可作为显式授权的交互式便利层，但不应成为脚本、CI 或 VS Code 正确性的依赖。

## 3. 最小必要设计

### 3.1 复用现有机制

- 保留现有 Git clone/worktree 拓扑，不迁移 common-dir，不接管 branch 创建。
- 保持 Cargo/Bazel 官方默认的 workspace 路径隔离，不新增全局 build cache 管理器。
- 复用现有 installer attestation 和 runner preflight，不引入后台 daemon、数据库或全局 registry。
- 用一个仓库内 Python CLI、一个版本化 marker schema 和现有测试框架完成闭环。

### 3.2 命令合同

计划中的稳定入口为：

```text
python3 scripts/workspace-safety/workspace_context.py bootstrap plan [--json]
python3 scripts/workspace-safety/workspace_context.py bootstrap apply [--plan <path>]
python3 scripts/workspace-safety/workspace_context.py doctor [--require-binary] [--json]
python3 scripts/workspace-safety/workspace_context.py exec -- <command> [args...]
```

- `plan` 只读，计算 canonical root、workspace id、branch、资源路径、冲突和 plan fingerprint。
- `apply` 重新验证 fingerprint，再创建/更新 marker、隔离目录和本地非秘密环境文件，最后自动 doctor。
- `doctor` 提供稳定诊断码，供人、Agent、脚本和 VS Code task 共用。
- `exec` 只修改子进程环境，不修改父 shell、用户 profile 或 Git config。

### 3.3 身份与状态

marker 至少记录：schema version、workspace id、canonical root、Git common-dir、remote URL SHA-256、当前 branch、状态版本和最近一次成功 doctor 摘要。

- workspace id 默认由 canonical root basename 派生；碰撞时 fail closed，不自动覆盖。
- branch 名变化使状态 `Stale`；同 branch 内 commit 前进不失效。
- detached HEAD 可运行 plan，但不能进入 `Ready`。
- 对相同 workspace/branch 重复 apply 必须幂等。
- branch 切换后复用该 workspace 的 runtime home 与 binary slot，只更新 branch 绑定；并行开发不同 branch 时应使用不同 worktree。

### 3.4 资源边界

| 资源 | 规则 | 共享策略 |
| --- | --- | --- |
| Git source/index/refs | 保持现有 clone/worktree | Git 按自身语义管理 |
| Cargo target | workspace 默认 `target/` | 不跨 workspace 共享 |
| Cargo registry/Rustup | 用户级默认 | 可共享 |
| Bazel outputBase | 保持 canonical root 派生默认 | 不显式指向相同目录 |
| Whale mutable home | `${XDG_STATE_HOME:-$HOME/.local/state}/whalecode/workspaces/<id>/home` | 不共享 |
| SQLite home | 与 workspace `WHALE_HOME` 一致 | 不共享 |
| 开发 binary slot | `${XDG_DATA_HOME:-$HOME/.local/share}/whalecode/workspaces/<id>/bin` | 不共享 |
| plan/marker | workspace state root 下版本化文件 | 不写 repo，不含秘密 |
| API key / 登录凭据 | OS keyring 或进程环境 | 不复制到 marker/home 模板 |
| legacy `~/.whale` | 保持原样 | 不作为开发入口默认值 |

## 4. 轻量门禁边界

代码层只接入项目已有的统一入口：本地 installer、workspace wrapper、cache regression、benchmark/E2E runner、仓库推荐的 build/test task。检查仅解析 marker、canonical root 和当前 branch，必须在产生副作用或模型请求前完成。

以下内容不做重型控制：

- 不安装全局 Git hook 或 shell hook；
- 不包装系统 `git`、`cargo`、`bazel`；
- 不阻止开发者绕过项目入口直接执行底层命令；
- 不提供 `--no-verify` 绕过高风险 runner；
- 不把 direnv 设为必需依赖。

根级 `AGENTS.md` 必须用强制语句要求 Agent 在修改、构建、测试或运行前执行 doctor；未 bootstrap 或 branch 已变化时先完成 plan/apply。README 和开发 runbook 提供同样的人类流程，相关子目录 AGENTS 只在存在更严格入口时补充，不复制整段规则。

## 5. 工作单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W0 | 固化通用资源基线 | observability | `scripts/workspace-safety/`、`docs/v0.0.5/local-workspace-safety/evidence/` | inventory schema 与只读扫描命令 | 新增不读取秘密内容的 workspace/Git/build/runtime/binary inventory | 任意 clone/worktree 都能生成同结构的环境事实 | 避免把当前两个目录的偶然路径写成通用设计 | Complexity: +1 只读命令与 evidence schema；Reach/Cost: 扫描本机路径和进程 metadata，不运行模型 | fake clone、linked worktree、两套 common-dir fixtures；当前两 workspace 输出与 Git 命令一致；隐私扫描 | 输出含秘密或路径结论不一致即停止；不提交本机原始 evidence | planned |
| W1 | 实现只读 bootstrap plan | internal | `scripts/workspace-safety/workspace_context.py` | `bootstrap plan`、plan schema、fingerprint | 解析 canonical root/workspace id/current branch/common-dir/resource paths/conflicts，并输出人类摘要与 JSON | 开发者在落地前知道检查结论和完整拟执行动作 | 提供可审查、可自动化的开工前检查，不因 plan 改变环境 | Complexity: +1 CLI 子命令和 plan schema，无新依赖；Reach/Cost: 每次新 branch/workspace 多一个显式步骤 | 文件/Git/environment 快照证明 plan 零写入；JSON schema、credential redaction、detached HEAD 和 collision fixtures | 任一读取不确定或发现冲突时只返回非零状态，不写 marker | planned |
| W2 | 建立 workspace/branch 状态机 | data | `workspace_context.py`、workspace XDG state root | versioned marker 与 `Unbootstrapped/Ready/Stale/Conflict/DoctorFailed` | 写入 canonical root、common-dir、remote digest、branch 和 doctor 摘要；branch 变化标记 Stale | 同一目录切换 branch 后不能沿用旧确认，commit 前进不触发误失效 | 落实“每个 branch 开工前检查”，同时避免 branch 级目录膨胀 | Complexity: +1 marker schema和5个机械状态；Reach/Cost: 每个 workspace 一个小状态文件，schema升级需兼容测试 | apply/重复 apply/branch switch/switch back/root move/common-dir change fixtures；状态转换断言 | marker 冲突不覆盖；单提交 revert，新增空状态目录移入回收站 | planned |
| W3 | 实现 bootstrap apply 闭环 | internal | `workspace_context.py bootstrap apply` | plan fingerprint、目录创建、本地环境文件 | 校验 plan 未过期，创建 `0700` state/home 与 data/bin，写非秘密配置并自动调用 doctor | 一次确认即可得到可用且被验证的隔离 workspace | 避免“只登记未验证”的半成品工作区 | Complexity: +1 有限写路径和幂等分支；Reach/Cost: 本机增加独立状态/数据目录与少量磁盘占用 | temp XDG root 下验证幂等、权限、plan/apply 间 branch 变化、部分目录恢复、legacy mtime/hash不变 | 任一关键写入失败进入 DoctorFailed，不修改 repo/Git/profile；新目录可安全移入备份 | planned |
| W4 | 隔离子进程运行时 | runtime | `workspace_context.py exec` | child environment contract | 设置 workspace-specific `WHALE_HOME`、`CODEX_SQLITE_HOME`、metadata 和已验证 binary slot PATH，不修改父进程 | workspace 的 SQLite、sessions、logs、skills、tmp 和 binary 解析彼此隔离 | 消除不同开发版本的状态污染并提高测试归因 | Complexity: +1 exec 路径，复用 marker；Reach/Cost: workspace 状态占用独立磁盘，配置不再自动同步 | 双 workspace 并发无模型 writer fixture；路径/inode/WAL/log无交叉；父环境不变 | 未 Ready 或 binary不匹配时不启动子进程，不 fallback 到全局 PATH | planned |
| W5 | 建立快速 doctor 与基础门禁 | security | `workspace_context.py doctor`、统一 task/preflight helper | stable diagnostic codes 与 fast marker check | doctor 深查资源；统一入口在副作用前快速校验 root/branch/state，输出恢复命令 | 人和 Agent 能在错误 workspace 或切 branch 后及时停止 | 把主要规范变成轻量、可诊断的工程兜底 | Complexity: +1 深查矩阵和1个快速检查函数；Reach/Cost: 受控入口增加毫秒级 preflight及测试维护 | 每个状态/错误码正反例；受控入口在创建文件、安装、模型请求前失败；直接 cargo 不受拦截 | 误报时停止接入该入口并修正检查，不扩大 allowlist或增加 bypass | planned |
| W6 | 隔离开发 binary 安装 | deployment | `scripts/install-whale-local.sh`、`scripts/test-install-whale-local.sh`、attestation | install scope 与 workspace destination | 要求显式 workspace/user scope；workspace scope 写当前 XDG binary slot并绑定 repo/tree/hash | 各 workspace 安装互不覆盖，legacy 用户安装仅由显式 promotion 更新 | 消除 codex workspace 误跑 alpha binary 的直接风险 | Complexity: +1 scope分支，复用现有 attestation；Reach/Cost: 旧无scope调用失败，installer测试面扩大 | fake HOME/XDG 双 workspace 安装；hash/attestation归属、原子替换和跨slot不变断言 | legacy `~/.whale/bin` 保持不动；新 slot可移入备份；逐提交revert | planned |
| W7 | 迁移高风险 runner | internal | cache regression、benchmark/E2E、仓库推荐 build/test tasks | binary/home resolution 与 bootstrap preflight | 删除开发入口对 `~/.whale/bin/whale` 的默认依赖，复用 doctor/attestation解析当前 slot | runner 在错误主体上于零请求、零副作用前失败 | 防止产生归属错误的付费、缓存和端到端证据 | Complexity: 修改既有入口，不新增runner；Reach/Cost: 调用方需先bootstrap或传显式路径，回归测试范围扩大 | parser/fixture测试；codex传alpha attestation必须零请求失败；缓存敏感变更执行 index gate | 逐入口迁移；任何调用契约不清晰则暂停该入口，不保留静默fallback | planned |
| W8 | 固化 Agent 与开发者规范 | documentation | `AGENTS.md`、`README.md`、`docs/runbooks/development-workflow.md`、workspace runbook、VS Code tasks | 开工前置规则、命令示例、恢复流程 | 增加一致的 bootstrap/doctor 说明；AGENTS 使用强制语句，VS Code task 调用权威 CLI | 新 workspace 和 Agent 会话无需依赖维护者口头提醒 | 让通用方案随仓库传播并成为可执行开发惯例 | Complexity: 更新4类入口文档和可选tasks，不复制实现；Reach/Cost: 命令变化时需同步维护，Agent开工增加一次检查 | 链接/命令 smoke；从 README 与 AGENTS 按步骤在临时 worktree完成bootstrap；术语一致性扫描 | VS Code wiring不稳定时单独revert，CLI/runbook保持权威 | planned |
| W9 | 防止共享资源回归 | developer-tooling | `scripts/workspace-safety/check_workspace_references.py`、现有本地/CI门禁 | forbidden default rules 与窄 allowlist | 扫描新的全局 whale默认、共享target/outputBase和未门禁高风险入口 | 后续分支不能无意恢复已关闭的串扰路径 | 把一次性治理变成持续、低成本的仓库合同 | Complexity: +1静态规则脚本和小allowlist；Reach/Cost: 脚本/文档调整可能触发规则，需维护精确匹配 | 正反fixtures；当前repo零未解释违规；规则单测 | 误报修正规则，不扩大allowlist掩盖执行入口 | planned |
| W10 | Bootstrap 两个现有 workspace | deployment | 当前 codex/alpha 的 XDG state/data roots | 两套 marker、home、binary slot | 确认无活动 Whale进程后分别 plan/apply，不复制legacy状态 | 当前两个目录成为通用流程的首批真实兼容样本 | 解除已知本机串扰，并验证方案不依赖同一Git拓扑 | Complexity: +2 marker和两套空隔离目录；Reach/Cost: 非秘密配置需分别维护，磁盘占用增加 | 两边doctor通过；legacy tree抽样不变；并发无模型smoke；0模型请求 | 任一环境无法安全初始化则只暂停该workspace；legacy不动，新目录移入备份 | planned |
| W11 | Windows parity | compatibility | PowerShell installer、doctor、tasks | Windows state/data/bin mapping | Linux稳定后另立专项实现 `%LOCALAPPDATA%` 与 `.exe` attestation | Windows后续获得等价工作流 | 避免在无Windows验证环境时复制Linux假设 | Complexity: 后续增加PowerShell实现；Reach/Cost: 需要Windows实机/CI，当前不阻塞Linux | Windows专项自动与实机验证 | 保持 deferred，不表述为已支持 | deferred |

## 6. 分阶段执行

### Phase 1：只读计划与身份状态

- Entry condition：工作树 clean；不运行安装、产品或模型。
- Work units：W0、W1、W2。
- Phase-local evidence：通用 inventory fixtures、plan 零写入证据、branch 状态转换测试。
- Cross-unit side effects：只增加 CLI/测试；W2 的 marker 写入只发生在 apply 测试的临时 XDG 根。
- Next-phase condition：clone 与 linked worktree 均能稳定区分 Unbootstrapped、Ready、Stale、Conflict。

### Phase 2：Apply、运行隔离与轻量门禁

- Entry condition：Phase 1 schema和状态机验证通过。
- Work units：W3、W4、W5。
- Phase-local evidence：apply 幂等/恢复、双 workspace 并发无模型 fixture、统一入口副作用前阻断。
- Cross-unit side effects：项目推荐入口开始要求 bootstrap；每个 workspace增加独立状态与数据目录。
- Next-phase condition：bootstrap 能从 plan 闭环到 doctor；branch 切换后所有受控入口稳定 fail closed。

### Phase 3：安装与 runner 迁移

- Entry condition：Phase 2 doctor和exec合同稳定。
- Work units：W6、W7、W9。
- Phase-local evidence：双slot安装、错误attestation零请求失败、静态回归门禁。
- Cross-unit side effects：旧无scope安装和依赖global whale的命令会失败，需要同步调用方。
- Next-phase condition：仓库高风险入口不再默认解析 legacy binary/home；cache-sensitive gate按现有规则通过。

### Phase 4：规范发布与现有 workspace 启用

- Entry condition：前三阶段自动测试通过；legacy `~/.whale` 保持原样。
- Work units：W8、W10。
- Phase-local evidence：README/AGENTS/runbook演练、codex/alpha doctor、并发无模型smoke。
- Cross-unit side effects：开发者和Agent开工增加显式plan/apply步骤；两个workspace分别维护非秘密配置。
- Next-phase condition：用户确认开发体验后，workspace 串扰阻塞关闭；不自动恢复其他0.146资格项。

## 7. 验证矩阵

### 7.1 Bootstrap 生命周期

```bash
python3 scripts/workspace-safety/workspace_context.py bootstrap plan --json
python3 scripts/workspace-safety/workspace_context.py bootstrap apply
python3 scripts/workspace-safety/workspace_context.py doctor --require-binary
```

断言：plan 零写入；apply 幂等；marker绑定canonical root和当前branch；branch切换后Stale；重新apply后Ready并复用原home/bin。

### 7.2 Git、build 与 runtime

```bash
git rev-parse --show-toplevel
git rev-parse --git-common-dir
git branch --show-current
cargo metadata --no-deps --format-version 1
bazel info output_base
```

断言：不同 workspace 的 source/index/target/outputBase/runtime home/binary slot符合资源边界；用户级registry/toolchain可共享；detached HEAD不能Ready。

### 7.3 仓库门禁

```bash
python3 -m unittest discover -s scripts/workspace-safety/tests -p 'test_*.py'
bash scripts/test-install-whale-local.sh
python3 scripts/workspace-safety/check_workspace_references.py
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
git diff --check
```

cache index gate 仅在触及既有敏感面时按项目规则执行。所有本计划自动测试均为无模型测试，不启动真实 Whale Agent run。

## 8. 风险与安全停止

| Risk | Trigger Signal | Mitigation | Safe Stop / Fallback |
| --- | --- | --- | --- |
| 误把 branch 当 workspace 身份 | 切换 branch 创建大量目录或同名branch碰撞 | canonical root为主身份，branch只作为可失效登记 | 停止apply，保留只读plan，修正marker逻辑 |
| plan/apply 竞态 | 两阶段之间root、branch或common-dir变化 | fingerprint绑定关键事实，apply重新解析 | 拒绝过期plan，要求重新plan |
| legacy 数据被误迁移 | 新home出现旧auth/session/history | apply只创建空目录和非秘密配置 | 立即停止；legacy不动，新目录移入备份 |
| 轻量门禁被绕过 | 直接cargo/git未检查bootstrap | AGENTS/README强制规范，统一入口基础兜底 | 接受底层工具边界；不扩张为系统级hook |
| 错 binary 被调用 | attestation主体或hash不符 | doctor与runner前置校验，不fallback到PATH | 零请求前失败，停止对应runner迁移 |
| build cache交叉 | 两workspace target/outputBase realpath相同 | 保持官方默认，不设置共享override | 移除override；不自动删除已有缓存 |
| 配置与磁盘增长 | 每workspace独立home/bin | 只隔离可变状态，保留registry/toolchain共享 | 报告大小；清理由用户另行授权 |

## 9. 明确不采用

| 方案 | 原因 |
| --- | --- |
| 一条命令接管branch/worktree创建 | 与VS Code、Git CLI和现有工作流耦合，不是必要条件 |
| 仅靠README、无机械状态 | Agent和runner无法可靠判断是否完成前置动作 |
| 全局Git/shell hook强制拦截 | 控制面过重、可移植性差，超出用户确认的基础门禁 |
| 每个branch创建一套runtime目录 | 切换频繁时产生大量重复状态；用户已选择workspace目录复用 |
| 首次bootstrap后永久有效 | 无法落实每个branch开工前检查 |
| 共享`~/.whale`并依赖SQLite WAL | 不解决不同版本的schema和状态语义污染 |
| 立即统一现有Git common-dir | 不能解决runtime根因，并引入不必要迁移风险 |
| daemon、全局registry或强制direnv | 当前文件marker和wrapper足以闭环，没有新增基础设施的收益依据 |

## 10. 验收与执行授权边界

- [ ] PRD中的五项用户决策均有实现与测试映射；
- [ ] 新clone、linked worktree和现有两套独立common-dir均可bootstrap；
- [ ] plan只读、apply幂等、doctor有稳定诊断码；
- [ ] branch变化使状态Stale，重新apply复用workspace目录；
- [ ] AGENTS、README、runbook和VS Code入口使用一致命令；
- [ ] 统一入口实施基础门禁，直接底层工具不被系统级拦截；
- [ ] runtime、SQLite、binary和高风险runner不跨workspace；
- [ ] legacy数据与凭据未迁移、未删除、未进入Git；
- [ ] 当前codex/alpha只作为首批验证样本，不成为实现特例；
- [ ] Windows保持deferred；
- [ ] 无真实模型请求，若未来触发仍遵守全局run ledger与预算门禁；
- [ ] 代码与文档按小主题提交、push，工作树最终clean。

本文件仅授权工程设计，不授权开始实现。执行时按 Phase 1 起步，每个小主题独立验证、commit并push；每次代码变更完成后按项目规则询问是否需要对抗性审查。
