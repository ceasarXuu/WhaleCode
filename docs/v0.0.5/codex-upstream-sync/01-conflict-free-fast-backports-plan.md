# 第一批：完全无文本冲突的快速 backport 合入方案

> 历史计划：本批已完成，仅保留执行证据，不再授权后续工作。当前唯一有效计划见 [plan.md](plan.md)。

- 文档状态：代码 backport 已完成；Linux 验证完成，Windows 动态验证与 TUI 基线债务待关闭
- 计划模式：Execution Tracking
- 创建日期：2026-08-01
- 适用版本：WhaleCode v0.0.5
- 计划基线：`d4be990a332d934884b872951f6f1e6a831e8e1f`
- 上游范围：OpenAI Codex `fed0a8f4..rust-v0.146.0`
- 关联分析：[Codex CLI 上游追赶差异分析与合并策略](README.md)

## 1. 目标

在不触及 DeepSeek、TaskSpace、provider payload、cache、protocol、state schema 或权限架构迁移的前提下，先回移六个可独立应用的上游修复。

本批次解决三个具体问题：

1. 收紧 Git 和 Windows PowerShell 命令安全识别，减少只读命令被仓库配置或大小写变体绕过审批边界的风险；
2. 禁止 Git 元数据读取触发 repository `core.fsmonitor` helper；
3. 吸收三个不改变外部协议的 TUI 正确性与分配优化。

## 2. “完全无冲突”的准入定义

提交只有同时满足下列条件才进入本批次：

1. `git apply --check --directory=third_party/codex-cli` 通过；
2. 每个目标文件与 Whale vendor 固定基线 `fed0a8f4` 内容完全一致；
3. 不修改 `Cargo.toml`、`Cargo.lock`、公共 schema 或生成物；
4. 不命中[缓存敏感面合同](../../../benchmarks/cache-regression/cache-surface-contract.json)中的生产路径规则；
5. 不引用当前旧基线不存在的新 crate、类型、feature 或迁移；
6. 可单提交、单测试、单独 `git revert`；
7. 不需要真实 Whale Agent run。

这里的“无冲突”仅表示已经排除已知文本冲突和 Whale 产品语义交叉。代码仍必须通过编译、测试和平台 smoke，才能在 Execution Tracking 中标记为 `verified`。

## 3. 已核实的准入证据

截至计划基线，六个提交的目标文件全部为 `UNCHANGED_FROM_VENDOR_BASE`，六个 patch apply check 全部为 `PASS`。当前执行：

```text
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
```

结果为 `PASS`，指纹未变化；全局事实仍保留“最近一次 live 回归失败”，本批次不得借此声称真实缓存回归已经恢复。

| ID | 上游提交 | 目标文件 | Patch SHA-256 | 准入结果 |
| --- | --- | --- | --- | --- |
| B1 | [`2e598df6`](https://github.com/openai/codex/commit/2e598df6fcd30717cfdcd2a898746a84d365ca23) | `shell-command/.../is_dangerous_command.rs`、`is_safe_command.rs` | `6e593ec541721139ed4f244d7b5925790e6396f5cd9a6016014c1111d2de0e3a` | 文本无冲突 |
| B2 | [`9deb4f9c8`](https://github.com/openai/codex/commit/9deb4f9c86426c40ba1e189831d7bc3634dd7b94) | `shell-command/.../windows_dangerous_commands.rs` | `0f9a74b890a4a92105884cd6e8a656ca0d4b9390f1837f280271e1d2ac75f2a1` | 文本无冲突 |
| B3 | [`6ec8c4a6`](https://github.com/openai/codex/commit/6ec8c4a6ecb17bc3ab10d0c5edf75494b50cab7e) | `git-utils/src/info.rs`、`core/src/git_info_tests.rs` | `57439b55804786783390df8467fdb6ff8b2159c3697cf4c25aa658e36ed124f2` | 文本无冲突 |
| B4 | [`36912ce3`](https://github.com/openai/codex/commit/36912ce3de1c039f7faaddd509d0465ff644e6c1) | `tui/src/bottom_pane/paste_burst.rs` | `93e3d717b9e72929d03c37a582bc18f487886d6f3e5b807656e41fc439eea4db` | 文本无冲突 |
| B5 | [`5d7e6a25`](https://github.com/openai/codex/commit/5d7e6a2503fc71f09cea71bfca9e193e0c3fd215) | `tui/src/wrapping.rs` | `a2aa6e1517fac5702b8a9ed5e96b3d28df1fce2394e0ecd84182b9646de3dbb8` | 文本无冲突 |
| B6 | [`c86b1be3`](https://github.com/openai/codex/commit/c86b1be3cdbe12307843bcc9e7a44c1904ddcdf1) | `tui/src/diff_render.rs` | `ce04a70932ffde4ed5af4687a20570325a6857859e1830154f878f5d754c29c3` | 文本无冲突 |

## 4. 技术合入方式

Codex 以 tarball vendor 方式进入 `third_party/codex-cli/`，官方提交与 Whale 根仓库没有 merge-base。本批次不用普通 cherry-pick，而是把官方提交的原始 binary patch 加上 vendor 目录前缀后应用。

每个提交使用同一闭环：

```bash
upstream_sha=<40-char-sha>

git cat-file -e "$upstream_sha^{commit}"
git show "$upstream_sha" --format= --binary \
  | git apply --check --directory=third_party/codex-cli
git show "$upstream_sha" --format= --binary \
  | git apply --directory=third_party/codex-cli
```

如果对象不在本地对象库，先从 `https://github.com/openai/codex.git` 获取指定 SHA；不得添加或改写项目长期 remote 配置，也不得把 `main` 的其他提交一起带入。

每次应用后必须：

1. 用 `git diff -- <精确目标文件>` 确认只有官方 patch 内容；
2. 运行该工作单元的 focused tests；
3. 运行所属 crate 回归；
4. `git add` 后执行 `git diff --cached --check`；
5. 执行缓存 index gate，确认本次暂存集未误触敏感面；
6. 使用独立本地提交，commit body 记录 `Upstream-Commit` 和 `Patch-SHA256`；
7. 立即 push 当前 `whalecode-codex` 分支；
8. 下一单元开始前确认工作树 clean、HEAD 与远端一致。

## 5. 工作单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W0 | 冻结批次输入 | compatibility | 六个官方 commit 与目标路径 | upstream SHA、patch digest、vendor-base equality | 在实施前重跑对象存在、SHA-256、基线 equality 和 apply check | 执行输入与本文记录一致 | 防止上游 ref 漂移或本地新改动让“无冲突”结论失效 | 六项检查结果与第 3 节完全一致 | 任一不一致即停止，不应用任何 patch | verified |
| W1 | 禁止 `git -C` 自动批准 | security | `codex-rs/shell-command/src/command_safety/` | `git_global_option_requires_prompt()`、`is_known_safe_command()` | 应用 `2e598df6`，把 `-C` 和 `-C<path>` 归入必须提示的 Git 全局选项 | 即使子命令看似只读，带 `-C` 的 Git 命令也不走已知安全自动批准 | 阻止仓库 `core.fsmonitor` 等配置借目录切换在无确认时执行 helper | 两个新增测试 + `cargo test -p codex-shell-command --lib` | focused/full test 任一失败则不提交；已提交后用 `git revert <W1-local-commit>` | verified `44e48e210` |
| W2 | 识别混合大小写 URL | security | `codex-rs/shell-command/src/command_safety/windows_dangerous_commands.rs` | `looks_like_url()` | 应用 `9deb4f9c8`，在定位 URL prefix 时进行 ASCII lowercase 比较并保留原始 slice | `HTTP://`、`hTtPs://` 等变体进入与小写 URL 相同的危险命令判断 | 降低 Windows PowerShell URL 大小写变体绕过安全判断的风险 | 新增 mixed-case 测试 + `cargo test -p codex-shell-command --lib` | 测试失败则不提交；已提交后单独 revert W2 | merged `4162a91c9`；Windows test pending |
| W3 | 禁止元数据读取启动 fsmonitor | security | `codex-rs/git-utils/src/info.rs`、`codex-rs/core/src/git_info_tests.rs` | `run_git_command_with_timeout()` | 应用 `6ec8c4a6`，为 Git metadata 命令固定追加 `-c core.fsmonitor=false` | Whale 的工作树状态采集不会执行 repository 自定义 fsmonitor helper | 降低打开陌生仓库时的隐式代码执行面，并减少元数据诊断副作用 | Unix helper marker 测试、`cargo test -p codex-git-utils --lib`、相关 `codex-core` test | 非 Git、clean、tracked/untracked 现有用例回归失败即停止；已提交后单独 revert W3 | verified `a5670f9a6` |
| W4 | 统一 Windows paste burst 判定窗口 | client | `codex-rs/tui/src/bottom_pane/paste_burst.rs` | `PASTE_BURST_CHAR_INTERVAL` | 应用 `36912ce3`，移除 Windows 30ms 特例，统一为 8ms | Windows 慢速键入更不容易被误判为 paste burst | 减少 VS Code/Windows 终端中正常输入被缓冲或吞键的用户体验问题 | `cargo test -p codex-tui paste_burst --lib`、`cargo test -p codex-tui --lib`、Windows TUI smoke | Windows smoke 未执行时批次保持未验证；失败则 revert W4 | merged `00b9d9006`；Windows smoke pending |
| W5 | 安全处理外部 borrowed slice | internal | `codex-rs/tui/src/wrapping.rs` | `borrowed_slice_range()` 与 wrapped range 计算 | 应用 `5d7e6a25`，先验证 slice 是否属于源文本，不属于时走已有 range mapping | wrapping 不再对外部 slice 做无效 pointer offset | 避免未定义行为，并让外部/合成 slice 的终端换行可预测 | 新增 rejection test、`cargo test -p codex-tui wrapping --lib`、`cargo test -p codex-tui --lib` | 任一 wrapping regression 失败即停止；已提交后单独 revert W5 | focused verified `595ff6d37` |
| W6 | 消除 diff render 的 FileChange clone | performance | `codex-rs/tui/src/diff_render.rs` | `Row<'a>`、`collect_rows()`、`line_counts()`、`DiffSummary -> Renderable` | 应用 `c86b1be3`，列表渲染借用 change，消费式渲染移动 change，并集中行数计算 | diff 展示保持排序和内容不变，同时不再为每个文件复制完整 diff 内容 | 大 diff 场景减少不必要内存复制；收益来自明确移除 clone，不声明未经测量的延迟百分比 | `cargo test -p codex-tui diff_render --lib`、snapshot review、`cargo test -p codex-tui --lib` | snapshot 内容变化或排序变化即停止；已提交后单独 revert W6 | focused verified `2c81aca14` |
| W7 | 验证批次集成 | compatibility | `third_party/codex-cli/codex-rs` workspace | 六个本地 backport commits | 在六项独立提交后执行格式、workspace check、CLI build、缓存 gate 和 Git clean 检查 | 六项可共同编译，Whale CLI 仍可构建，未误触 cache contract | 在进入下一上游批次前提供可复现的稳定检查点 | 第 7 节全部通过，HEAD 与远端一致 | 失败时定位首个引入提交并使用 `git revert`，不使用 reset | partial：TUI baseline 33 failures |
| W8 | 固化追溯与失败原因 | observability | `docs/migration/codex-sync/`、本计划 | 新 dated sync log、work-unit execution table | 记录每个 upstream/local commit、patch digest、测试输出、平台缺口、revert 记录和最终状态 | 后续 vendor refresh 可识别已回移提交，失败无需重新调查 | 减少重复 backport 和故障归因成本，为下一批次提供审计证据 | 链接、commit、命令结果与 Git 历史逐项一致 | 只记录事实；验证未通过时标记 blocked/failed，不修改成乐观结论 | verified |

## 6. 实施阶段

### Phase 1：安全边界修复

- Entry condition：W0 六项证据保持成立，工作树 clean。
- Work units：W1、W2、W3。
- Phase-local evidence：三项分别完成 focused/full crate tests，并形成三个独立已推送提交。
- Next-phase condition：命令安全和 Git metadata 现有回归无失败；无缓存敏感面变化。

### Phase 2：TUI 叶子修复

- Entry condition：Phase 1 的三个提交均可在远端定位。
- Work units：W4、W5、W6。
- Phase-local evidence：每项 focused test、TUI lib test、snapshot review；W4 另有 Windows smoke。
- Next-phase condition：TUI snapshots 没有未解释变化，Windows paste 行为得到验证或被明确标记为 blocker。

### Phase 3：批次闭环

- Entry condition：六个代码单元都已有独立提交和测试证据。
- Work units：W7、W8。
- Phase-local evidence：workspace/CLI 验证、cache gate、sync log、clean Git 状态。
- Next-phase condition：本批次 execution status 可以依据真实证据更新；不自动授权下一架构批次。

## 7. 精确验证命令

所有 Cargo 命令从 `third_party/codex-cli/codex-rs` 执行。

### 7.1 W1：`git -C` 审批

```bash
cargo test -p codex-shell-command git_dash_c_requires_prompt --lib
cargo test -p codex-shell-command git_global_override_flags_are_not_safe --lib
cargo test -p codex-shell-command --lib
```

通过标准：`git -C . status`、`git -C. status` 和 shell-wrapped 变体均不是 known-safe；普通只读 `git branch --show-current` 保持安全。

### 7.2 W2：Windows URL 大小写

```bash
cargo test -p codex-shell-command powershell_start_process_mixed_case_urls_are_dangerous --lib
cargo test -p codex-shell-command --lib
```

通过标准：混合大小写 HTTP/HTTPS 都被判断为危险；本地非 URL `Start-Process` 现有反例保持通过。

### 7.3 W3：Git fsmonitor 隔离

```bash
cargo test -p codex-core test_get_has_changes_ignores_repo_fsmonitor_config --lib
cargo test -p codex-core test_get_has_changes --lib
cargo test -p codex-git-utils --lib
```

通过标准：marker helper 未执行；non-git、clean、tracked change、untracked change 的返回值保持原语义。

### 7.4 W4：paste burst

```bash
cargo test -p codex-tui paste_burst --lib
cargo test -p codex-tui --lib
```

Windows smoke：在 VS Code integrated terminal 和一个原生 Windows terminal 中分别验证正常快速键入、短文本粘贴、长文本粘贴、Enter 提交和 Ctrl+C 恢复；不得出现正常输入被延迟、吞键或错误占位符。

### 7.5 W5：wrapping

```bash
cargo test -p codex-tui borrowed_slice_range_rejects_slices_outside_source_text --lib
cargo test -p codex-tui wrapping --lib
cargo test -p codex-tui --lib
```

通过标准：新增外部 slice 用例通过，现有 Unicode、indent、owned/borrowed range 和 textarea wrapping 测试无回归。

### 7.6 W6：diff render

```bash
cargo test -p codex-tui diff_render --lib
cargo test -p codex-tui ui_snapshot_diff_gallery --lib
cargo test -p codex-tui --lib
```

通过标准：文件排序、rename destination highlighting、added/removed 计数和 gallery snapshots 不变；不得通过接受未知 snapshot 更新绕过。

### 7.7 批次闭环

```bash
cargo fmt --all -- --check
cargo check -p codex-cli --bin whale --locked
cargo test -p codex-shell-command --lib
cargo test -p codex-git-utils --lib
cargo test -p codex-tui --lib
git diff --check
git status --short --branch
```

缓存门禁必须另从仓库根执行：

```bash
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
```

脚本按当前工作目录解析仓库根；即使从 `codex-rs` 能通过相对路径启动脚本，也会错误地在该目录下查找 `benchmarks/`。

## 8. 提交、推送与回滚

每个工作单元单独提交并 push，不创建新分支。建议提交主题：

| Work Unit | Local commit subject |
| --- | --- |
| W1 | `fix(upstream): require approval for git -C` |
| W2 | `fix(upstream): detect mixed-case URLs on Windows` |
| W3 | `fix(upstream): disable repo fsmonitor for metadata reads` |
| W4 | `fix(upstream): align Windows paste burst interval` |
| W5 | `fix(upstream): validate borrowed wrapping slices` |
| W6 | `perf(upstream): avoid cloning TUI file changes` |
| W8 | `docs(v0.0.5): record conflict-free upstream backports` |

回滚只使用可审计的 `git revert <local-commit>`，不得使用 `git reset --hard` 或覆盖 vendor 文件。若后续工作单元依赖失败单元，先停止后续执行，再按逆序 revert 已依赖的本批次提交。

## 9. 可观测性与证据

本批次不新增 provider/session/protocol 日志字段，因为六项均为叶子级安全、Git metadata 或 TUI 行为；新增这些字段反而会扩大缓存和协议表面。

使用以下现有或新增证据：

- 安全判断由现有 approval 行为和 command-safety tests 观察；
- fsmonitor 通过不可出现的 marker 文件证明 helper 未执行；
- TUI 通过 unit tests、snapshots 和 Windows smoke 观察；
- 每项 upstream/local commit 和测试结果写入 codex-sync log；
- 命令失败必须保存命令、exit code 和首个失败测试名称，不以“环境问题”代替失败原因。

## 10. 明确排除项

| 候选 | 排除原因 | 后续归属 |
| --- | --- | --- |
| `3afb185a`、`2dbde94a` network proxy 安全修复 | 是否启用 managed proxy 路径仍需产品/runtime 判断 | network proxy 专项批次 |
| `a14a73b54` long proxy socket path | 依赖 proxy routing 架构上下文 | network proxy 专项批次 |
| `2e0c4f497` `/diff` repository config 安全修复 | 当前 patch apply check 失败 | TUI/Git 安全人工迁移批次 |
| permission profiles | 变更配置、sandbox 和公共协议 | permission/config 架构迁移批次 |
| MCP、Skills、Plugins、Code Mode | 会改变 tool catalog 或 provider-visible context | capability/cache 批次 |
| DeepSeek、TaskSpace、protocol、state、compaction | 命中项目核心差异和缓存敏感面 | 独立高风险迁移批次 |
| `main` 中 0.147 alpha 提交 | 不属于稳定目标 | 后续稳定版本评估 |

## 11. 风险

| Risk | Trigger Signal | Mitigation | Safe Stop / Fallback |
| --- | --- | --- | --- |
| 文本可应用但隐含依赖缺失 | compile error 指向新类型、trait 或 feature | 每项先 focused compile/test，不批量应用 | 不提交该项，保留其他独立项 |
| W1 增加正常 Git 命令提示 | 现有 known-safe tests 或手工 smoke 出现非 `-C` 行为变化 | 只接受官方精确 patch，不扩大 Git option 范围 | revert W1 |
| W3 影响 fsmonitor 仓库状态判断 | clean/dirty 测试返回值改变 | 同时跑四类 `get_has_changes` 回归 | revert W3 |
| W4 的 8ms 阈值不适配实际 Windows terminal | 正常键入被缓冲或粘贴识别失败 | Windows 双终端 smoke 作为批次关闭条件 | revert W4，不阻塞 W5/W6 |
| W6 改变 diff 顺序或 snapshot | snapshot 或 rename/highlight 测试变化 | 禁止无解释更新 snapshots | revert W6 |
| 文档声称超出证据 | 未运行平台验证却标记 verified | execution table 区分 not-started/in-progress/verified/blocked | 保持未验证或 blocked |

## 12. 批次验收标准

- 六个上游 patch 各有一个独立、已推送的本地 commit；
- 每个 commit body 可追溯到完整 upstream SHA 和 patch SHA-256；
- 所有 focused tests 和所属 crate tests 通过；
- W4 完成 Windows 双终端 smoke；
- `cargo fmt --check`、Whale CLI check 和缓存 index gate 通过；
- 没有新增 cache-sensitive、DeepSeek 或 TaskSpace 变化；
- 没有未解释的 snapshot 更新；
- codex-sync log 记录成功、失败、跳过和回滚事实；
- 工作树 clean，当前 HEAD 与 `origin/whalecode-codex` 一致。

## 13. 官方资料

1. [OpenAI Codex CLI 0.146.0 Release](https://github.com/openai/codex/releases/tag/rust-v0.146.0)
2. [上游 `git -C` 审批安全修复](https://github.com/openai/codex/commit/2e598df6fcd30717cfdcd2a898746a84d365ca23)
3. [上游 Windows 混合大小写 URL 修复](https://github.com/openai/codex/commit/9deb4f9c86426c40ba1e189831d7bc3634dd7b94)
4. [上游 fsmonitor 隔离修复](https://github.com/openai/codex/commit/6ec8c4a6ecb17bc3ab10d0c5edf75494b50cab7e)
5. [上游 Windows paste burst 修复](https://github.com/openai/codex/commit/36912ce3de1c039f7faaddd509d0465ff644e6c1)
6. [上游 borrowed wrapping slice 修复](https://github.com/openai/codex/commit/5d7e6a2503fc71f09cea71bfca9e193e0c3fd215)
7. [上游 TUI diff clone 优化](https://github.com/openai/codex/commit/c86b1be3cdbe12307843bcc9e7a44c1904ddcdf1)
