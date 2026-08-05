# 本机多分支与多工作空间安全管理工程计划

- 文档状态：计划完成，待执行授权
- 计划模式：Plan Authoring
- 创建日期：2026-08-06
- 适用版本：WhaleCode v0.0.5
- 当前工作空间：`$HOME/whalecode-codex` / `whalecode-codex`
- 同机工作空间：`$HOME/whalecode-alpha` / `whalecode-alpha`
- 范围：Linux 本机开发、构建、CLI 安装、无模型测试与运行时状态隔离

## 1. 问题与结论

两个目录当前没有共享 Git common-dir、index、HEAD 或 Cargo target，因此源码编辑、提交和常规 Cargo 构建不会互相覆盖。真正的风险来自 Git 之外的用户级可变资源：

- `WHALE_HOME` 和 `CODEX_SQLITE_HOME` 未设置时，两边都写 `~/.whale`；
- PATH 上的 `whale` 指向 `~/.whale/bin/whale`，当前安装证明显示它来自 `whalecode-alpha`，且已落后于 alpha 当前 HEAD；
- cache regression 默认选择 `~/.whale/bin/whale`，可在 codex 工作空间误跑 alpha 二进制；
- SQLite、sessions、history、logs、plugins、skills、shell snapshots 和临时执行目录都在共享 home 下；
- 两边共用 GitHub remote，但属于两套独立 object/ref 数据库，某一边 fetch 不会更新另一边的 remote-tracking refs。

目标不是引入新的通用 workspace manager，而是建立一个窄而明确的安全合同：

> 每个 canonical repository root 对应唯一 workspace identity、唯一可变运行时 home、唯一开发二进制槽位；所有有副作用的开发命令必须显式解析并验证这三个对象，找不到或不匹配时 fail closed。

## 2. 官方与成熟实践依据

1. [Git worktree 官方文档](https://git-scm.com/docs/git-worktree.html)说明 linked worktree 共享 common repository，但 HEAD、index 和 worktree metadata 分离；同一 common-dir 内 Git 会拒绝把同一 branch 普通检出到两个 worktree。它也提供 `worktreeConfig`，但开启后会改变配置读取规则，旧 Git 可能拒绝访问。
2. [Cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html#buildtarget-dir)和[Cargo build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html)规定默认 target 位于 workspace root；跨 workspace 共享编译缓存应使用专门的 `sccache`，而不是让不同源码树直接共用一个 target-dir。
3. [Bazel output directory layout](https://bazel.build/remote/output-directories)默认以 canonical workspace root 路径的 MD5 生成独立 outputBase；只有显式指定相同 `--output_base`/`--output_user_root` 才会破坏这一隔离。
4. [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)把持久状态放入 `XDG_STATE_HOME`、非必要缓存放入 `XDG_CACHE_HOME`、进程间运行时对象放入 `XDG_RUNTIME_DIR`，并要求新建用户状态目录采用 `0700`。
5. [SQLite WAL 文档](https://sqlite.org/wal.html)说明 WAL 允许 reader/writer 并发，但同一 WAL 同时只有一个 writer。即便文件锁正确，不同代码版本共享一份会迁移和解释 schema 的状态库仍不是安全的 workspace 隔离。
6. [direnv](https://direnv.net/)是成熟的目录级环境加载方案，进入目录加载、离开目录卸载，并要求显式 `allow`。本机当前未安装，因此本计划只把它作为人工 shell 的可选增强，不把它设为脚本、CI 或 VSCode task 的正确性依赖。

## 3. 设计决策

### 3.1 保留现有 Git 拓扑

本批不把 alpha 迁入当前 `$HOME/WhaleCode/.git`，也不重建任何 worktree：

- Git 不是当前串扰根因；
- alpha 使用 bare common-dir，而当前 worktree 使用 non-bare common-dir；统一会涉及重新检出、未跟踪构建物和本地引用迁移；
- `extensions.worktreeConfig` 会改变 `core.bare`/`core.worktree` 的配置语义，不值得为运行时隔离扩大 Git 变更面。

后续新建 workspace 时优先使用同一个受控 common repository 的 `git worktree add`；现有两个目录只登记和审计，不迁移。

### 3.2 Workspace identity

初始 `workspace_id` 使用 canonical repo root 的 basename：

```text
$HOME/whalecode-codex -> whalecode-codex
$HOME/whalecode-alpha -> whalecode-alpha
```

resolver 在 workspace state root 写 `workspace-identity.json`，至少记录：

- schema version；
- workspace id；
- canonical repo root；
- Git common-dir；
- remote URL 的 SHA-256，而不是凭据化 URL 原文。

若相同 id 已绑定其他 canonical root，命令必须失败，不自动复用或覆盖。目录移动后也必须显式重新登记，避免旧状态被静默接管。

### 3.3 资源边界

| 资源 | 推荐位置/规则 | 共享策略 |
| --- | --- | --- |
| Git source/index/refs | 保持现有 worktree/clone | 不跨目录写；remote 可相同 |
| Cargo final/intermediate artifacts | workspace 自己的 `target/` | 不设置全局 `CARGO_TARGET_DIR`；不跨 workspace 复用 target |
| Cargo registry/git cache/Rustup | 现有用户级默认 | 可共享；只承担下载缓存和工具链职责 |
| Bazel outputBase | 保持按 workspace root 自动计算 | 禁止两个 workspace 指向同一显式 outputBase |
| Whale mutable home | `${XDG_STATE_HOME:-$HOME/.local/state}/whalecode/workspaces/<id>/home` | 严禁共享 |
| SQLite home | 与该 workspace 的 `WHALE_HOME` 相同 | 严禁共享 |
| 开发二进制槽位 | `${XDG_DATA_HOME:-$HOME/.local/share}/whalecode/workspaces/<id>/bin` | 严禁覆盖其他 id |
| 临时执行、sessions、history、logs | 位于隔离的 `WHALE_HOME` 下 | 严禁共享 |
| API key / 登录凭据 | OS keyring 或启动进程环境 | 不写 repo；不通过复制/软链整个 home 共享 |
| `~/.whale` | legacy/user-promoted 安装与历史状态 | 不再作为开发 runner 默认值；本批不删除、不迁移 |

当前 Whale 尚未把 config/data/state 全部分拆为不同 XDG API，因此本批通过现有 `WHALE_HOME` 完成粗粒度隔离，不借机重构运行时目录模型。

### 3.4 命令入口

权威入口采用仓库内、可测试的 workspace wrapper，而不是依赖用户是否正确激活 shell：

```text
workspace_context.py bootstrap
workspace_context.py doctor [--require-binary]
workspace_context.py exec -- <command> [args...]
```

`exec` 只为子进程设置 `WHALE_HOME`、`CODEX_SQLITE_HOME` 和 workspace metadata，并把当前 workspace 的已验证 binary slot置于子进程 PATH最前，不修改全局 shell/profile。人工终端后续可选择 direnv，但所有 installer、benchmark、cache regression 和 VSCode task 仍必须调用 wrapper 或传显式 binary/home。

## 4. 非目标

- 不迁移、删除或合并两个 Git worktree/common-dir；
- 不清理 `~/.whale`、Cargo cache、Bazel cache、node_modules 或任一 target；
- 不自动复制 auth、credentials、history、sessions、plugins 或 skills；
- 不让两个开发版本共享 SQLite，即使 WAL 支持文件级并发；
- 不引入 daemon、数据库、全局 workspace registry、容器平台或新的 package manager；
- 不启用 `sccache`，除非后续有构建性能数据证明需要；
- 不在本批执行真实 Whale Agent run 或模型请求；
- Windows workspace parity 继续按既有决定 deferred，本批只保证 Linux 本机路径。

## 5. 工作单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W0 | 固化本机资源事实 | observability | `docs/v0.0.5/local-workspace-safety/`、新 audit 输出 | 两个 repo root、Git common-dir、binary attestation、runtime home、build roots | 增加只读 inventory 命令和规范化 JSON evidence，不读取 config/auth 内容 | 每次实施前能证明当前共享面、活动进程和 workspace clean 状态 | 防止依据过期路径或错误二进制实施隔离 | Complexity: +1 只读审计入口和 evidence schema；Reach/Cost: 扫描 Git/path/process metadata，不运行产品或模型 | fixture 测试；在两目录运行后 path/common-dir/branch/HEAD 与 Git 命令一致；隐私扫描 | 审计含敏感值或事实不一致即停止；删除未提交 evidence 即回滚 | planned |
| W1 | 建立稳定 workspace identity | internal | `scripts/workspace-safety/workspace_context.py` | `resolve_workspace()`、`workspace-identity.json` | 从 canonical root basename 解析 id，在 XDG state 下创建 `0700` workspace root，并以 marker 绑定 repo root/common-dir/remote digest | 相同 id 不能静默接管其他目录的状态 | 为后续 home、binary 和门禁提供单一身份事实，降低拿错目录的诊断成本 | Complexity: +1 Python CLI、1 marker schema，无服务/依赖；Reach/Cost: 每个 workspace 增加一个小型 state root和维护命令 | temp fake repo 正反例；重复 id/移动目录/common-dir 变化均 fail closed；目录权限断言 `0700` | marker 冲突不改任何文件；单提交 revert，已创建空目录可移入回收站 | planned |
| W2 | 隔离运行时状态 | runtime | `workspace_context.py exec` | child process environment | 为子进程设置 workspace-specific `WHALE_HOME`、`CODEX_SQLITE_HOME` 和 identity，并把已验证 binary slot置于子进程 PATH最前，不改父 shell与 profile | 两个版本的 SQLite、sessions、history、logs、plugins、skills、tmp 和命令解析分开 | 避免不同 schema/行为版本污染同一运行记录或执行错误binary，提升测试归因 | Complexity: +1 exec 子命令和环境合同；Reach/Cost: 每个 workspace 独立状态占用磁盘，配置和 skills 不再自动同步 | 同时启动两个无模型 fixture writer，验证路径、inode、SQLite/WAL、日志和 `command -v whale` 无交叉；退出后父环境未变化 | wrapper 失败则不启动子进程；回滚到 W1 只读 doctor，不回退共享运行时执行 | planned |
| W3 | 隔离开发二进制安装 | deployment | `scripts/install-whale-local.sh`、`scripts/test-install-whale-local.sh`、现有 attestation | install scope、destination、attestation | 要求显式 `--scope worktree` 或 `--scope user`；worktree scope 安装到 XDG data slot且禁止改 PATH，user scope 才允许触碰 legacy `~/.whale/bin` | alpha/codex 安装互不覆盖，用户级 `whale` 只能通过显式 promotion 更新 | 消除当前最直接的“在 codex 目录执行 alpha binary”风险 | Complexity: +1 scope 参数和原子安装/校验分支，复用现有 attestation；Reach/Cost: 现有无 scope 安装命令会 fail closed，需要更新文档和调用方 | fake HOME 下双 workspace 安装；hash/attestation repo_root 对应；第二次安装不改变另一 slot；user scope 缺显式 flag失败 | 保留现有 `~/.whale/bin` 不动；新 slot 可整体移入回收站；revert installer 提交 | planned |
| W4 | 建立 workspace doctor 门禁 | security | `workspace_context.py doctor`、tests | stable diagnostic codes | 校验 home/SQLite/binary slot/attestation/Cargo target/Bazel override/PATH resolution，并输出不含秘密的 JSON 和人类摘要 | 错 workspace、共享 home、全局 stale binary、跨目录 target 在运行前被阻断 | 把隐性环境串扰变成可定位错误，减少误测和状态损坏 | Complexity: +1 检查矩阵和稳定错误码；Reach/Cost: 本地命令增加毫秒级 preflight，需维护允许的临时 target 边界 | 每个违规条件一项负例；本机 codex 当前应准确报告 global alpha binary 和 shared legacy home；0 secret/path credential 泄漏 | doctor 只读；误报时停在诊断层，不允许 `--no-verify` 绕过高风险 runner | planned |
| W5 | 移除高风险 runner 的全局默认 | internal | `scripts/cache-regression/run_cache_hit_regression.py`、`.ps1`、`scripts/run-deepseek-reasoning-replay-e2e.ps1`、相关 harness | `--whale-bin` default 与 binary health preflight | 删除 `~/.whale/bin/whale` 开发默认；从当前 workspace slot解析或要求显式路径，并复用 attestation 验证 repo_root/tree/hash | benchmark/cache/E2E 不会因 PATH 顺序使用另一个 workspace 的二进制 | 保护付费、缓存和端到端证据的主体身份，避免产生不可用结果 | Complexity: 修改 3 类入口及测试，不新增 runner；Reach/Cost: 旧调用需补 bootstrap/显式参数，真实模型仍受原预算门禁 | parser/fixture 测试；故意传 alpha attestation 给 codex runner必须在 0 请求前失败；正确 slot通过 preflight | 任一调用方尚依赖全局默认则停止迁移该入口；逐 runner 单提交 revert | planned |
| W6 | 防止新的共享资源回归 | developer-tooling | `scripts/workspace-safety/check_workspace_references.py`、CI/本地门禁 | forbidden default/reference rules | 扫描生产脚本中新出现的 `~/.whale/bin/whale` 默认、固定共享 `CARGO_TARGET_DIR`/Bazel outputBase、未包装的高风险 `whale` 解析；使用窄 allowlist记录 legacy 文档 | 新 runner 不能重新引入已关闭的串扰路径 | 让隔离合同随代码演进持续生效，而非一次性本机配置 | Complexity: +1 静态规则脚本和小型 allowlist；Reach/Cost: 脚本变更增加门禁维护，文档/evidence不作为执行入口 | 正反 fixture；对当前 repo 运行 0 未解释违规；规则本身单测 | 规则误报先调整精确匹配，不扩大 allowlist掩盖真实执行入口 | planned |
| W7 | 安全启用两个现有 workspace | data | XDG state/data workspace roots、legacy `~/.whale` | codex/alpha identity markers 与空隔离 home | 先检查无活动 Whale 进程，再分别 bootstrap；不复制 legacy 状态，只验证现有 credentials来源可由安全环境/keyring提供 | 两个 workspace 从空、独立状态开始，legacy 数据保持原样可回看 | 避免自动迁移不可逆地混合 schema、秘密或历史，同时快速解除当前阻塞 | Complexity: +2 identity marker和两套空 home；Reach/Cost: 用户需要分别配置非秘密选项，磁盘状态增长，旧 session不自动出现 | bootstrap/doctor 两边通过；legacy tree hash/mtime抽样不变；无 auth/config内容进入 Git；0 模型请求 | 任一凭据或必要配置无法安全提供则暂停对应 workspace运行；legacy保持不动，空 home可移入回收站 | planned |
| W8 | 验证并形成运行手册 | documentation | `docs/development/local-workspace-safety.md`、VSCode tasks/说明 | build/run/install/cache/e2e workflows | 记录显式命令，VSCode task调用 wrapper；并发运行无模型 smoke，验证 Git/build/state/binary矩阵 | 开发者能在正确 workspace 重复执行，不依赖记忆或 shell残留 | 降低本机维护和交接成本，给后续 0.146 qualification提供可信环境 | Complexity: +1 runbook和必要 task wiring；Reach/Cost: 文档/VSCode入口需随命令更新，不安装 direnv依赖 | 两个 VSCode terminal/task分别 doctor；并发 fixture smoke；Cargo target/Bazel outputBase、binary hash、state writes全部隔离；工作树clean/push | VSCode变量解析不稳定时保留wrapper命令为权威入口，task wiring可单独revert | planned |
| W9 | Windows parity | compatibility | PowerShell installer、doctor、tasks | Windows state/data/bin mapping | 在 Linux方案稳定后另立专项映射 `%LOCALAPPDATA%`、PowerShell环境和 `.exe` attestation | Windows最终获得同等隔离 | 避免在未验证Windows环境时复制Linux路径假设 | Complexity: 后续增加PowerShell实现和Windows测试；Reach/Cost: 需要Windows实机/CI，当前不阻断Linux本机 | Windows专项自动与实机验证 | 按既有用户决定保持 deferred，不在本批声称支持 | deferred |

## 6. 分阶段执行

### Phase 1：身份和只读门禁

- Entry condition：两个工作树 clean；无活动 `whale`/`whale-app-server`；不运行安装或模型。
- Work units：W0、W1、W4。
- Phase-local evidence：双 workspace inventory、identity fixture、doctor 正反例。
- Cross-unit side effects：只新增工具、测试和空 state marker；不改变现有 CLI、PATH 或 `~/.whale`。
- Next-phase condition：doctor 能稳定识别当前 shared home 和 alpha-installed global binary，并拒绝 identity collision。

### Phase 2：运行时与二进制隔离

- Entry condition：Phase 1 通过；workspace root 权限和 marker 已验证。
- Work units：W2、W3、W5、W6。
- Phase-local evidence：双 slot 安装、attestation 主体匹配、runner 在模型请求前拒绝错误 binary、静态回归门禁。
- Cross-unit side effects：安装命令必须显式 scope，旧开发命令会 fail closed；每个 workspace开始承担独立状态和安装空间。
- Next-phase condition：仓库内高风险 runner 不再默认解析 `~/.whale/bin/whale`，两个 workspace的 write set 不重叠。

### Phase 3：本机启用与收口

- Entry condition：Phase 2 全部通过；用户确认 legacy `~/.whale` 保持原样且不自动迁移。
- Work units：W7、W8。
- Phase-local evidence：两个真实 workspace doctor、无模型并发 smoke、runbook、VSCode task。
- Cross-unit side effects：用户需要为两个 workspace分别维护非秘密配置；legacy `whale` 仍存在但不参与开发证据。
- Next-phase condition：用户确认后才恢复 0.146 qualification blocker 修复；仍不授权真实模型 run。

## 7. 精确验证矩阵

### 7.1 Git 与 build

```bash
git status --short --branch
git rev-parse --git-dir
git rev-parse --git-common-dir
git worktree list --porcelain

test -z "${CARGO_TARGET_DIR:-}"
cargo metadata --no-deps --format-version 1
bazel info output_base
```

断言：两 workspace repo root、Git metadata和默认 build roots符合登记事实；Bazel outputBase不同；不得通过设置同一个全局 `CARGO_TARGET_DIR` 获得“加速”。

### 7.2 Identity、home 与 binary

```bash
python3 scripts/workspace-safety/workspace_context.py bootstrap
python3 scripts/workspace-safety/workspace_context.py doctor --require-binary
python3 scripts/workspace-safety/workspace_context.py exec -- whale --version
```

断言：

- codex/alpha workspace id、state root、binary slot 均不同；
- `WHALE_HOME == CODEX_SQLITE_HOME` 且属于当前 id；
- binary attestation 的 `repo_root`、tree、hash 对应当前 workspace；
- PATH 中存在 legacy `whale` 不影响 wrapper选择；
- marker/attestation/log不含 API key、credential或完整敏感环境。

### 7.3 并发无模型 smoke

同时在两个 workspace wrapper 下运行只写本地状态的 fixture，不调用模型。验证：

- SQLite/WAL、history、session、log、tmp 路径没有交集；
- 两个 writer 不触碰 legacy `~/.whale`；
- 停止其中一个不删除或改写另一个的状态；
- `pgrep`/open-file evidence 能按 workspace home归属进程。

### 7.4 仓库门禁

```bash
python3 -m unittest discover -s scripts/workspace-safety/tests -p 'test_*.py'
bash scripts/test-install-whale-local.sh
python3 scripts/workspace-safety/check_workspace_references.py
git diff --check
git status --short --branch
```

若变更触及 cache-sensitive path，仍必须执行现有 cache index gate；本计划不得以 workspace smoke 替代真实 cache regression，也不得绕过真实运行预算门禁。

## 8. 风险与安全停止条件

| Risk | Trigger Signal | Mitigation | Safe Stop / Fallback |
| --- | --- | --- | --- |
| workspace id 碰撞 | marker指向不同 canonical root | basename + marker绑定，禁止自动覆盖 | 停止bootstrap；重命名目录或用户明确选择新id |
| 隔离后配置漂移 | 两 workspace行为因 config不同而不可比 | 比较非秘密配置摘要；测试报告记录workspace id和config digest | 不复制秘密/状态；先补显式测试配置 |
| legacy状态误迁移 | 新home出现旧sessions/auth/history | bootstrap只创建空目录；迁移不在本批 | 停止运行，将新home移入回收站，legacy不动 |
| 错误binary仍被调用 | attestation repo_root/tree/hash不符 | runner前置doctor并删除全局默认 | 0请求前fail closed，不允许fallback到PATH |
| build cache交叉污染 | 两边target或Bazel outputBase realpath相同 | 保持官方默认workspace隔离 | 停止并移除环境override；不删除缓存 |
| 自动环境工具失效 | VSCode/direnv未加载环境 | wrapper是权威入口，环境工具只做便利层 | task直接调用wrapper；不依赖父shell |
| 磁盘增长 | 两套target/state超出预期 | 分别统计，缓存清理由后续显式任务处理 | 不自动删除；报告路径和大小后由用户决定 |

## 9. 被拒绝的方案

| 方案 | 不采用原因 |
| --- | --- |
| 让两个分支继续共享 `~/.whale`，依赖 SQLite WAL | WAL只解决文件级并发，不解决不同代码版本的schema迁移和状态语义 |
| 让两个 workspace共享一个 Cargo target | 编译期绝对路径、features、build scripts和临时源码生命周期会产生污染；此前0.146 qualification已实际踩中这一类问题 |
| 立即统一两套 Git common-dir | 不能解决runtime根因，且扩大到worktree重建、refs和未跟踪构建物迁移 |
| 把 branch name 当唯一身份 | branch可切换，且两套独立common-dir无法阻止同名branch；canonical root + marker更稳定 |
| 自动复制或软链 credentials/config/sessions | 泄密、schema和状态污染风险高，回滚边界不清晰 |
| 强制安装 direnv | 本机未安装，脚本/CI/VSCode也不能把正确性建立在交互式shell hook上 |
| 新建后台workspace daemon/registry | 当前只有两个本机workspace，文件marker和wrapper已足够，没有持续服务的收益依据 |

## 10. 验收清单

- [ ] 两个 workspace identity 唯一且绑定 canonical root；
- [ ] `WHALE_HOME`、SQLite、sessions、logs、tmp无路径交集；
- [ ] worktree binary slot唯一，attestation对应当前repo/tree/hash；
- [ ] 开发 installer必须显式scope，不覆盖其他workspace；
- [ ] cache/benchmark/E2E不再默认使用 `~/.whale/bin/whale`；
- [ ] Cargo target与Bazel outputBase保持workspace隔离；
- [ ] legacy `~/.whale`未删除、未自动迁移、未被smoke写入；
- [ ] credentials不进入repo、marker、日志或attestation；
- [ ] 并发无模型smoke通过，未启动真实Whale Agent run；
- [ ] Linux runbook和VSCode入口可复现；
- [ ] Windows明确保持deferred，不被表述为验证通过；
- [ ] 所有代码和文档提交、push，工作树clean。

本计划完成后，只解除“同机 workspace 串扰”这一工程阻塞；它不改变 DeepSeek、TaskSpace 或 Codex upstream 迁移设计，也不自动恢复 0.146 cutover 资格。
