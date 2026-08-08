# 通用 Branch/Workspace Bootstrap 安全管理工程计划

- 文档状态：Phase 1、Phase 2 已验证；Phase 3 及后续工作单元待授权
- 计划模式：Plan Authoring + 独立 Execution Tracking
- 创建日期：2026-08-06
- 更新日期：2026-08-08
- 适用版本：WhaleCode v0.0.5
- 产品基线：`prd/2026-08-06-workspace-bootstrap.md`
- 首批验证对象：`$HOME/whalecode-codex`、`$HOME/whalecode-alpha`
- 范围：Linux 本机开发、构建、测试、CLI 安装和运行时状态隔离

## 1. 问题与目标合同

现有两个 workspace 没有共享 Git common-dir、index、HEAD 或默认 Cargo target；当前串扰来自 Git 之外的用户级可变资源：默认 `~/.whale`、SQLite、sessions、logs、plugins、skills、临时目录、PATH 上的全局 `whale` 和 runner 的默认 binary 解析。

本方案建立所有未来 WhaleCode workspace 共用的生命周期合同：

> branch 被检出到实际 workspace 后，开发者或 Coding Agent 必须先显式执行 `bootstrap plan → bootstrap apply`。canonical workspace root 是稳定身份，当前 branch 是可失效登记；branch 切换后重新 bootstrap，但复用 workspace 已隔离的运行时和二进制目录。

纯 Git ref 不触发资源创建。项目统一入口实施轻量 fail-closed 检查，README、AGENTS 和 runbook 承担主要流程治理；不拦截开发者直接调用 Git/Cargo 等底层工具。

## 2. 已知事实、约束与外部依据

### 2.1 已知事实

- `whalecode-codex` 与 `whalecode-alpha` 属于两套 Git common-dir，源码编辑和提交不会彼此覆盖。
- 两边未显式设置时都会使用 `~/.whale`；PATH 上的 legacy `whale` 来自 alpha 的旧安装。
- Python 与 PowerShell cache regression runner 仍默认选择 `~/.whale/bin/whale`。
- 仓库根没有 `Cargo.toml`；活动 Rust workspace 位于 `third_party/codex-cli/codex-rs/Cargo.toml`。
- Bazel module 位于 `third_party/codex-cli/MODULE.bazel`，不能从仓库根无条件执行 `bazel info`。
- 当前大量 benchmark/E2E 入口为 PowerShell；哪些属于 Linux 本批必须由 D0 先形成清单，不能预设。

### 2.2 工程约束

- Linux 首版；Windows/PowerShell 安装与运行入口保持 deferred。
- plan 必须零写入；apply 必须验证用户确认的同一份计划上下文。
- 不迁移、不覆盖、不删除 legacy `~/.whale`；不复制凭据、history、sessions、plugins 或 skills。
- 所有自动测试均为无模型测试；真实 Whale Agent run 仍受预算与全局账本门禁。
- 新功能必须有脱敏结构化日志、稳定诊断码、冒烟与回归测试。

### 2.3 外部依据

1. [Git worktree 官方文档](https://git-scm.com/docs/git-worktree.html)：linked worktree 共享 common repository，但 HEAD、index 和 worktree metadata 分离。
2. [Cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html#buildtarget-dir)与[Cargo build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html)：默认 target 位于 workspace root；跨 workspace 复用应使用专门缓存，而非共享 target-dir。
3. [Bazel output directory layout](https://bazel.build/remote/output-directories)：默认 outputBase 由 canonical workspace path 派生，显式共享 output root 才破坏隔离。
4. [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)：持久状态、数据、缓存和运行时对象应使用相应基础目录并限制状态目录权限。
5. [SQLite WAL](https://sqlite.org/wal.html)：WAL 改善并发，但不提供不同代码版本之间的 schema 与状态语义隔离。
6. [direnv](https://direnv.net/)：可作为显式授权的交互便利层，但不应成为脚本、CI 或 VS Code 正确性的依赖。

## 3. 最小必要设计

### 3.1 复用与删除

- 保留现有 Git clone/worktree 拓扑，不迁移 common-dir，不接管 branch 创建。
- 保持 Cargo/Bazel 官方默认的 workspace 路径隔离，不新增 build cache 管理器。
- 复用 installer attestation、cache runner preflight 和现有 Python unittest，不新增依赖。
- 删除开发入口对 legacy `~/.whale/bin/whale` 的默认解析，不增加兼容 fallback。
- 用一个仓库内 Python CLI、一个版本化 marker 和一个追加式审计日志完成闭环；不引入 hook、daemon、数据库或全局 registry。

### 3.2 命令与 plan/apply 合同

```text
python3 scripts/workspace-safety/workspace_context.py bootstrap plan [--json]
python3 scripts/workspace-safety/workspace_context.py bootstrap apply --expect <fingerprint>
python3 scripts/workspace-safety/workspace_context.py doctor [--require-binary] [--json]
python3 scripts/workspace-safety/workspace_context.py exec -- <command> [args...]
```

- `plan` 只向 stdout/stderr 输出人类摘要、canonical JSON 和 fingerprint，不创建 plan 文件或 marker。
- fingerprint 对 canonical JSON 做 SHA-256；输入至少包含 canonical root、workspace id、Git common-dir、当前 branch、目标资源路径、现有 marker 摘要和 schema version。
- `apply --expect` 在任何写入前重新计算计划；不匹配即拒绝，要求重新 plan。
- `apply` 只创建/更新允许的 XDG 目录、marker 和非秘密环境元数据，随后调用 doctor。
- `doctor` 输出稳定诊断码；`exec` 只修改子进程环境，不修改父 shell、用户 profile 或 Git config。

### 3.3 Identity 与状态

```text
display_name = canonical root basename
workspace_id = <sanitized-display-name>-<sha256(canonical-root)[0:10]>
```

marker 记录 schema version、workspace id、canonical root、Git common-dir、当前 branch、资源路径和最近一次成功 doctor 摘要。remote URL 不进入 marker；inventory 仅可输出去凭据后的 host/path 摘要。

持久状态只有 `Unbootstrapped`、`Ready`、`Stale`、`Conflict` 和 `DoctorFailed`。plan 结果不是持久状态。

- branch 名变化使当前检查为 Stale；同 branch 内 commit 前进不失效。
- detached HEAD 可运行 plan，但不能进入 Ready。
- 重复 apply 必须幂等；摘要碰撞或 marker 绑定其他 root 时 fail closed。
- canonical root 移动或重命名后生成新 workspace id并表现为Unbootstrapped；旧状态保留，不自动迁移。只有同一id位置出现绑定其他root的marker时才是Conflict。
- branch 切换后复用 workspace runtime home 与 binary slot；并行 branch 应使用不同 worktree。
- 轻量门禁只能识别“检查时的当前 branch 是否匹配”。A→B→A 且全程绕过统一入口的历史切换不追踪，这是不引入 Git hook/reflog 状态机的明确边界。

### 3.4 资源与权限

| 资源 | 规则 | 权限/共享策略 |
| --- | --- | --- |
| Git source/index/refs | 保持现有 clone/worktree | Git 自身管理 |
| Cargo target | `third_party/codex-cli/codex-rs/target` 或显式 workspace-local target | 不跨 workspace 共享 |
| Cargo registry/Rustup | 用户级默认 | 可共享 |
| Bazel outputBase | 仅在检测到 Bazel module 时检查官方路径派生 | 不显式共享 |
| Whale mutable/SQLite home | `${XDG_STATE_HOME:-$HOME/.local/state}/whalecode/workspaces/<id>/home` | 目录 `0700`，不共享 |
| marker/audit log | 同一 workspace state root | 目录 `0700`，文件 `0600` |
| 开发 binary slot | `${XDG_DATA_HOME:-$HOME/.local/share}/whalecode/workspaces/<id>/bin` | 目录 `0700`，binary 保留可执行位，不共享 |
| API key/登录凭据 | OS keyring 或进程环境 | 不写 marker、日志或环境模板 |
| legacy `~/.whale` | 保持原样 | 不作为开发入口默认值 |

## 4. 工作单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| D0 | 固化实际入口与构建根 | discovery | `scripts/`、`.vscode/`、`README.md`、`docs/runbooks/`、根及vendor构建清单 | Linux入口清单、build-root清单、共享资源引用清单 | 用 `rg`、manifest检测和只读解析生成规范化JSON到临时目录；标明每个入口的平台、副作用、binary/home解析和模型风险 | 后续单元只修改已证实的入口，未知PowerShell范围保持阻塞 | 防止通用计划依赖不存在的根Cargo/Bazel命令或漏掉付费入口 | Complexity: +1只读inventory命令/schema；Reach/Cost: 扫描仓库metadata，无运行时/模型成本，需维护schema | fixture覆盖clone/worktree/无Cargo/嵌套Cargo/Bazel可选；本机输出人工比对；隐私扫描 | 输出含秘密或分类不确定即停止；证据留临时目录，不提交本机绝对路径 | verified |
| W1 | 实现纯context resolver与plan | internal | `scripts/workspace-safety/workspace_context.py`、`scripts/workspace-safety/tests/test_plan.py` | `resolve_context()`、`build_plan()`、canonical JSON、fingerprint | 解析root/id/common-dir/branch/resource paths/marker摘要并只输出计划 | 任意workspace能在零写入下获得可确认计划 | 为人和Agent提供一致的开工检查，并关闭plan阶段副作用 | Complexity: +1 Python CLI和plan schema，无新依赖；Reach/Cost: 每次开工多一次本地解析 | fake repos正反例；文件/Git/environment快照不变；JSON确定性、脱敏与fingerprint测试 | 解析不确定或发现冲突时非零退出，不写任何状态 | verified |
| W2 | 实现纯identity与状态判断 | internal | `workspace_context.py`、`scripts/workspace-safety/tests/test_state.py` | `derive_workspace_id()`、`evaluate_state()`、marker schema | 实现basename+root摘要identity及五态纯函数，不执行文件写入 | 同名目录自动隔离，branch/common-dir/resource变化得到稳定状态；root移动形成新identity | 消除人工命名死路，并让状态机可独立审查测试 | Complexity: +2纯函数和1schema；Reach/Cost: marker升级需兼容测试，无运行IO路径 | 同名root、摘要碰撞、branch切换/切回、commit前进、root移动形成新id、detached HEAD fixtures | 任何碰撞返回Conflict；无文件可回滚 | verified |
| W3 | 实现幂等apply落盘 | data | `workspace_context.py`、`scripts/workspace-safety/tests/test_apply.py` | `apply_plan()`、atomic marker writer、XDG目录 | 校验`--expect`后原子创建权限受限目录和marker，再调用doctor；不写repo/Git/profile/legacy | 用户确认的上下文与实际写入严格一致，重复执行不产生第二套资源 | 关闭plan/apply竞态和半初始化风险 | Complexity: +1写路径和原子替换逻辑；Reach/Cost: 每workspace增加小型state/data目录和磁盘占用 | 临时XDG下测试过期fingerprint、幂等、权限、部分失败恢复、legacy hash/mtime不变 | fingerprint不符零写入；失败保留可诊断状态，新目录只能移入备份/回收站 | verified |
| W4 | 建立doctor与审计日志 | observability | `workspace_context.py`、`scripts/workspace-safety/tests/test_doctor.py` | `run_doctor()`、diagnostic codes、`workspace-events.jsonl` | 深查marker/root/branch/home/bin/attestation/build override；仅在apply已开始后及独立doctor/exec中追加脱敏机械事件，plan不落日志 | 成功、失败和失败阶段均可定位，Agent可按稳定码恢复 | 满足日志驱动要求并降低环境问题诊断成本，同时保持plan零写入 | Complexity: +1检查矩阵、诊断码表和JSONL；Reach/Cost: apply/doctor/exec增加少量本地IO，日志需轮转上限 | 每个诊断码正反例；证明plan不创建日志；字段allowlist、`0600`、单行损坏容忍和大小上限测试 | doctor只读除审计追加；日志失败不掩盖主诊断，禁止记录环境值 | verified |
| W5 | 隔离子进程环境 | runtime | `workspace_context.py`、`scripts/workspace-safety/tests/test_exec.py` | `exec_ready()`、managed child env override allowlist | 继承父环境副本，仅受控覆盖`WHALE_HOME`、`CODEX_SQLITE_HOME`、`CODEX_HOME`移除和当前bin PATH前缀后启动子进程 | SQLite、sessions、logs、skills、tmp和binary解析按workspace隔离，同时保留开发工具、代理和凭据环境 | 避免不同开发版本互相污染并提高测试归因 | Complexity: +1子进程入口；Reach/Cost: workspace状态独立占用磁盘，配置不自动同步 | 双workspace并发无模型writer；inode/WAL/log无交叉；父环境不变；仅四个受管键变化；Stale零启动 | 非Ready或binary不符时不启动，不fallback到PATH | verified |
| W6 | 提供可复用快速门禁 | security | `scripts/workspace-safety/workspace_context.py`、`scripts/workspace-safety/tests/test_require_ready.py` | `require-ready`子命令/函数 | 只校验marker、canonical root和current branch，返回稳定码与恢复命令 | 既有入口能用毫秒级检查阻断明显错workspace | 用窄机制支撑基础管控，不复制doctor全量逻辑 | Complexity: +1快速子命令；Reach/Cost: 每个接入入口增加两次只读Git查询和一次marker读取 | Ready/Unbootstrapped/Stale/Conflict/DoctorFailed fixtures、零写入和耗时记录；证明副作用前退出 | 误报先修复统一函数，不给调用方增加bypass | verified |
| W7 | 隔离Linux开发安装 | deployment | `scripts/install-whale-local.sh`、`scripts/test-install-whale-local.sh` | `--scope workspace`、`--scope user`、destination、attestation | 安装前调用require-ready；workspace scope写当前XDG slot，user scope才允许legacy路径 | Linux workspace安装互不覆盖，全局promotion保持显式 | 关闭当前codex误跑alpha binary的直接风险 | Complexity: +1scope分支，复用attestation；Reach/Cost: 无scope调用失败，installer测试面扩大 | fakeHOME/XDG双workspace安装；hash/attestation归属、原子替换、跨slot不变 | legacy不动；新slot移入备份；逐提交revert | planned |
| W8 | 迁移Linux cache runner | internal | `scripts/cache-regression/run_cache_hit_regression.py`、对应Python tests | `--whale-bin`解析与preflight | 删除`~/.whale/bin/whale`默认，默认从Ready workspace slot解析并验证attestation | cache run不会在错误binary上启动 | 保护付费与缓存证据主体身份，失败发生在零请求前 | Complexity: 修改1个Python入口和tests；Reach/Cost: 调用前需bootstrap，触及cache敏感面需执行index gate | parser/fixture；codex传alpha attestation零请求失败；cache index gate | preflight不稳定则暂停该runner迁移，不恢复global fallback | planned |
| W9a | 接入active-prefix宿主门禁 | internal | `scripts/taskspace-benchmark/run-active-prefix-matrix.py`、对应tests | `main()`在创建run root前的workspace与预算preflight | 在任何目录写入、Docker启动或模型请求前调用require-ready并验证现有运行授权合同 | active-prefix矩阵不会从错误workspace启动 | 确保付费证据可归属，并把环境错误提前到零请求阶段 | Complexity: 修改1个Python入口及tests；Reach/Cost: 旧直调需先bootstrap，真实运行仍受账本/预算约束 | plan-only与无模型fixtures；Stale时零目录/零Docker/零请求；预算合同回归 | 现有授权链不清晰则暂停，不以workspace门禁替代预算门禁 | planned |
| W9b | 接入provider-wire宿主门禁 | internal | `scripts/taskspace-benchmark/r7_a2_b0_provider_wire_probe.py`、对应tests | `main()`在创建raw/output和HTTP请求前的workspace与预算preflight | 在任何文件写入或provider请求前调用require-ready并验证现有运行授权合同 | provider-wire探针不能绕过workspace身份启动 | 防止高请求量探针生成主体不明、无法采信的provider证据 | Complexity: 修改1个Python入口及tests；Reach/Cost: 直接调用契约收紧，PowerShell封装需在W14另验 | 无模型HTTP stub；Stale时零文件/零请求；repeat×scenario预算负例 | 授权主体无法从现有wrapper证明时暂停，不自动发起真实probe | planned |
| W10 | 接入VS Code便利任务 | client | `.vscode/tasks.json`或D0确认的既有配置 | Bootstrap Plan/Apply、Doctor、Rust Check任务 | 仅调用权威CLI；Rust Check在`third_party/codex-cli/codex-rs`执行或传manifest-path | VS Code用户无需手工拼接环境即可走同一合同 | 降低日常使用摩擦，但不成为正确性唯一入口 | Complexity: +1编辑器配置文件/若已存在则增任务；Reach/Cost: VS Code用户受益，CLI用户不受影响，需维护变量展开 | task命令dry-run；临时worktree执行plan/doctor；Rust check路径解析正确 | VS Code变量不稳定时单独revert，CLI不受影响 | planned |
| W11 | 固化Agent和开发规范 | documentation | `AGENTS.md`、`README.md`、`docs/runbooks/development-workflow.md`、`docs/runbooks/local-workspace-safety.md` | 开工前置规则、命令、边界和恢复 | CLI达到W6后同步写入强制AGENTS声明与人类runbook，明确直接底层命令的责任边界 | 新Agent会话和开发者能重复执行正确流程 | 把基础门禁升级为仓库长期惯例，减少口头知识 | Complexity: 更新4类文档，不新增运行路径；Reach/Cost: 命令变更需同步维护，开工增加一次检查 | 链接与命令smoke；术语扫描；按文档在临时worktree完成plan/apply/doctor | 文档命令未通过smoke不合入；不提前声明未实现命令可用 | planned |
| W12 | 防止共享资源回归 | developer-tooling | `scripts/workspace-safety/check_workspace_references.py`、`scripts/workspace-safety/tests/test_reference_gate.py` | forbidden defaults与窄allowlist | 扫描新的legacy whale默认、共享target/outputBase和D0确认的未门禁入口 | 后续改动不能重新引入已关闭串扰 | 让隔离合同持续生效而非一次性配置 | Complexity: +1静态规则和小allowlist；Reach/Cost: 脚本文档调整可能触发规则，需维护精确匹配 | 正反fixtures；当前repo零未解释违规；allowlist每项写理由 | 误报修正规则，不扩大allowlist掩盖执行入口 | planned |
| W13 | Bootstrap现有两个workspace | deployment | codex/alpha XDG state/data roots | 两套marker/home/bin及并发证据 | 确认无活动Whale进程后分别plan/apply，不复制legacy状态 | 两种Git拓扑成为通用流程首批真实样本 | 解除已知本机串扰并验证无目录特例 | Complexity: +2marker和两套空目录；Reach/Cost: 非秘密配置分别维护，磁盘占用增加 | 两边doctor；legacy抽样不变；并发无模型smoke；0模型请求 | 任一workspace失败只暂停该对象；legacy不动，新目录移入备份 | planned |
| W14 | Windows与PowerShell parity | compatibility | `scripts/install-whale-local.ps1`、PowerShell cache/benchmark/tasks | Windows state/data/bin及attestation | Linux稳定后另立专项映射和测试 | Windows后续获得等价合同 | 避免无实机证据时混入Linux交付 | Complexity: 后续增加PowerShell实现；Reach/Cost: 需Windows实机/CI，当前不阻塞Linux | Windows专项自动与实机验证 | 保持deferred，不表述为已支持 | deferred |

## 5. 分阶段执行

### Phase 1：发现、只读计划与纯状态

- Entry condition：当前工作树 clean；不运行安装、产品或模型。
- Work units：D0、W1、W2。
- Phase-local evidence：实际入口清单、plan零写入、identity/state纯函数fixtures。
- Cross-unit side effects：只增加脚本与测试，不创建真实workspace marker。
- Next-phase condition：clone和linked worktree均能稳定输出确定性plan及五态判断；D0已把其余Linux宿主入口收敛为W9a/W9b。

### Phase 2：Apply、诊断与运行隔离

- Entry condition：Phase 1 schema与fingerprint合同通过。
- Work units：W3、W4、W5、W6。
- Phase-local evidence：过期fingerprint零写入、apply幂等、诊断日志脱敏、双workspace并发无模型fixture、快速门禁副作用前失败。
- Cross-unit side effects：每个测试workspace增加独立XDG状态；仍未修改installer或runner默认行为。
- Next-phase condition：bootstrap从plan闭环到Ready；branch变化后require-ready稳定拒绝。

### Phase 3：逐入口接入

- Entry condition：Phase 2 CLI与诊断码稳定。
- Work units：W7、W8、W9a、W9b、W12。
- Phase-local evidence：双slot安装、cache runner错误主体零请求失败、D0确认入口逐项证据、静态回归门禁。
- Cross-unit side effects：旧Linux无scope安装和global whale默认会失败；调用方需先bootstrap。
- Next-phase condition：已确认Linux高风险入口均不再默认解析legacy binary/home；cache-sensitive index gate通过。

### Phase 4：开发体验与本机启用

- Entry condition：前三阶段自动测试通过；legacy `~/.whale`保持原样。
- Work units：W10、W11、W13。
- Phase-local evidence：VS Code task、README/AGENTS/runbook演练、codex/alpha doctor和并发无模型smoke。
- Cross-unit side effects：开发者和Agent开工增加显式plan/apply；两个workspace分别维护非秘密配置。
- Next-phase condition：用户确认体验后关闭workspace串扰阻塞；不自动恢复其他0.146资格项。

## 6. 精确验证

### 6.1 生命周期

```bash
PLAN_JSON="$(python3 scripts/workspace-safety/workspace_context.py bootstrap plan --json)"
PLAN_FINGERPRINT="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["fingerprint"])' <<<"$PLAN_JSON")"
python3 scripts/workspace-safety/workspace_context.py bootstrap apply --expect "$PLAN_FINGERPRINT"
python3 scripts/workspace-safety/workspace_context.py doctor --require-binary
```

断言：plan前后repo/Git/XDG状态不变；apply只写allowlist路径；重复apply幂等；plan/apply间branch变化零写入失败；branch变化为Stale，重新apply复用原home/bin。

### 6.2 Git与构建根

```bash
git rev-parse --show-toplevel
git rev-parse --git-common-dir
git branch --show-current
cargo metadata --no-deps --format-version 1 \
  --manifest-path third_party/codex-cli/codex-rs/Cargo.toml
(cd third_party/codex-cli && bazel info output_base)
```

Bazel命令仅在`third_party/codex-cli/MODULE.bazel`存在且本机bazel可用时执行；否则doctor只检查显式共享override，不把缺少bazel视为失败。

### 6.3 自动门禁

```bash
python3 -m unittest discover -s scripts/workspace-safety/tests -p 'test_*.py'
bash scripts/test-install-whale-local.sh
python3 scripts/workspace-safety/check_workspace_references.py
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
git diff --check
```

cache index gate只在触及既有敏感面时执行；不得用workspace smoke替代真实cache regression，也不得绕过真实运行预算门禁。

## 7. 风险与安全停止

| Risk | Trigger Signal | Mitigation | Safe Stop / Fallback |
| --- | --- | --- | --- |
| plan/apply竞态 | fingerprint不匹配 | apply重算并在写入前比较 | 零写入拒绝，重新plan |
| identity摘要碰撞 | 同id绑定不同canonical root | basename+10位root摘要并校验完整root | Conflict，不自动覆盖或fallback |
| legacy数据误迁移 | 新home出现旧auth/session/history | apply写入allowlist，测试legacy hash/mtime | 停止；legacy不动，新目录移入备份 |
| secret进入状态/日志 | 输出包含URL凭据或环境值 | remote不进marker，字段allowlist和redaction测试 | 阻断提交并清理未提交测试状态 |
| 轻量门禁被绕过 | 直接cargo/git未检查 | AGENTS/README约束，统一入口兜底 | 接受明确边界，不扩张为hook/reflog状态机 |
| 错binary被调用 | attestation主体/hash不符 | doctor和runner preflight，不fallback到PATH | 零请求前失败，暂停对应入口 |
| build cache交叉 | 两workspace target/outputBase realpath相同 | 保持官方默认，不设置共享override | 移除override，不自动删除缓存 |
| inventory启发式漂移 | 新脚本未被识别或测试/库误报为入口 | 只把可执行入口纳入entrypoints，引用仍全量保留；W12固化规则fixtures | 发现错分先修inventory，不据此自动执行任何入口 |
| 磁盘增长 | workspace独立home/bin/log | 状态日志设上限，只共享registry/toolchain | 报告路径/大小；清理由用户另行授权 |

## 8. 不采用的方案

| 方案 | 原因 |
| --- | --- |
| 接管branch/worktree创建 | 与VS Code、Git CLI和现有流程耦合，不是必要条件 |
| plan落盘形成Planned状态 | 破坏只读合同并增加临时状态清理；fingerprint显式传递已足够 |
| 用户手工解决同名目录 | 通用bootstrap应自动产生稳定identity；root摘要成本更低 |
| remote URL摘要进入marker | 不是隔离所需身份，且增加凭据派生值与remote变更语义 |
| 全局Git/shell hook或reflog状态机 | 控制过重，超出已确认的基础门禁；历史切换盲区作为明确边界 |
| 每branch创建一套runtime目录 | 产生重复状态；用户已选择workspace目录复用 |
| 共享`~/.whale`并依赖SQLite WAL | 不解决不同版本schema和状态语义污染 |
| 统一现有Git common-dir | 不解决runtime根因并引入迁移风险 |
| daemon、全局registry或强制direnv | 文件marker与wrapper已足够，没有基础设施收益依据 |

## 9. 验收与授权边界

- [ ] plan零写入，apply强制验证`--expect`且过期计划零写入失败；
- [ ] workspace id自动消除同名目录冲突，marker不保存remote URL或其凭据派生摘要；
- [ ] marker状态只有Unbootstrapped、Ready、Stale、Conflict、DoctorFailed；
- [ ] apply幂等，doctor有稳定诊断码与脱敏审计日志；
- [ ] 新clone、linked worktree和两套独立common-dir均可bootstrap；
- [ ] branch变化使状态Stale，重新apply复用workspace目录；历史切换盲区被明确接受；
- [ ] Cargo验证使用实际manifest，Bazel仅在适用且可用时执行；
- [ ] installer、cache runner、active-prefix和provider-wire宿主入口分别接入基础门禁；容器内部组件由宿主入口负责；
- [ ] AGENTS、README、runbook与VS Code task使用一致命令；
- [ ] legacy数据与凭据未迁移、未删除、未进入Git或日志；
- [ ] 当前codex/alpha只作为首批样本，不成为实现特例；
- [ ] Windows/PowerShell保持deferred；
- [ ] 无真实模型请求，未来真实运行仍遵守run ledger与预算门禁；
- [ ] 代码和文档按小主题提交、push，工作树最终clean。

## 10. Execution Tracking

| Work Unit | Execution Status | Evidence | Missing Evidence | Decision |
| --- | --- | --- | --- | --- |
| D0 | verified | `workspace_inventory.py`、schema、4项fixture、当前workspace脱敏盘点；详见`02-d0-entrypoint-inventory-report.md` | 运行时行为只做静态归类，不声称真实模型路径已验证 | 收口 |
| W1 | verified | `workspace_context.py` resolver/plan、plan schema、8项专用fixture、clean HEAD真实零写入smoke；提交`e48a9da68` | 无；fingerprint消费侧已由W3验证 | 收口 |
| W2 | verified | identity/state纯函数、marker schema、5项专用fixture；提交`5b27de45b` | 无；marker生产路径已由W3验证 | 收口 |
| W3 | verified | 原子marker、`0700/0600`权限、8项apply fixture；提交`3d3c335aa` | 当前真实workspace未apply，保留给W13 | 收口 |
| W4 | verified | doctor schema、稳定诊断码、有界脱敏JSONL、10项fixture；提交`93a73c702` | 未接入生产入口 | 收口 |
| W5 | verified | Ready-only exec、受管环境覆盖、双workspace并发fixture、5项测试；提交`95145de39` | installer尚未填充真实workspace binary slot | 收口 |
| W6 | verified | fast resolver、五态门禁、6项fixture、当前workspace 50次平均2.426 ms；提交`f6ca840fd` | 尚未接入installer/runner | 收口 |
| W7-W14 | not-started | 无生产接入证据 | 各单元计划中的实现与验证证据 | 未获本轮执行授权 |

Phase 1实施结果见`03-phase1-implementation-report.md`，Phase 2实施结果见`04-phase2-implementation-report.md`。本轮授权止于Phase 2，不自动授权W7及后续工作。后续每个代码主题独立验证、commit并push，完成后按项目规则询问是否需要对抗性审查。
