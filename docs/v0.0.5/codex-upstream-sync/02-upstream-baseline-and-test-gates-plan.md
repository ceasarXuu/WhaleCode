# 第二批：上游基线、overlay 账本与测试门禁工程计划

- 文档状态：实施中；Phase 1 已验证，Phase 2/3 待执行
- 计划模式：Execution Tracking
- 创建日期：2026-08-01
- 适用版本：WhaleCode v0.0.5
- 计划基线：`fb31acff6362a67904d70fe6e7b9fc2f0c661135`
- vendor 固定基线：`fed0a8f4faa58db3138488cca77628c1d54a2cd8`
- 上游候选目标：Codex CLI `rust-v0.146.0` / `e363b08c9175ac1cbe5893615dd2cb9ddf95043b`
- 关联文档：[总体差异分析](README.md)、[第一批合入方案](01-conflict-free-fast-backports-plan.md)

## 1. 目标与结果定义

第二批不刷新 vendor，也不继续扩大功能 backport，而是先建立后续追赶所依赖的事实基线和测试门禁。完成后必须能够回答：

1. 当前 vendor 相对固定上游基线到底改了哪些路径，这些路径分别属于 Whale overlay、已回移上游修复还是尚未分类项；
2. 哪些上游提交已经回移到本仓库，如何证明没有重复或遗漏；
3. 当前 TUI 全量测试的失败集合是什么，哪些是快照差异、功能断言或环境问题；
4. 第一批 Windows 修复在哪些环境中完成了自动与人工验证；
5. 任意后续 vendor refresh 是否能通过一个本地、无模型费用、可重复运行的校验入口判断是否具备准入条件。

本批验收口径是：来源可追溯、overlay 可机器复现、backport 不重复、TUI 失败可稳定分类、Windows 证据不伪造。计划与工具不得把“未执行”写成“通过”。

## 2. 范围

### 2.1 本批包含

- 修正 `third_party/codex-cli/UPSTREAM.md` 中已经失真的 overlay 描述和数量；
- 生成确定性的 Whale overlay inventory；
- 建立选择性 backport 机器账本，并收录已有回移记录；
- 建立 metadata 一致性校验器；
- 用 Nextest 建立 TUI 可重复基线、失败分类和最终门禁；
- 逐组审阅并修复当前 TUI 快照或功能断言债务；
- 建立 Windows 自动验证脚本和两类终端 paste smoke 记录；
- 形成第二批执行记录，并逐主题提交、推送。

### 2.2 明确不包含

- 将 vendor 整体刷新到 `rust-v0.146.0`；
- network proxy 两个可选 backport；
- message-history、HTTP transport、app-server transport 或 permission profiles 迁移；
- DeepSeek provider、wire payload、缓存前缀或 TaskSpace schema/宿主挂点改造；
- 自动接受全部 Insta 快照；
- 真实 Whale Agent run 或任何模型费用；
- 在没有 Windows 环境证据时把 Windows 项标记为 verified。

若实施中发现必须触及上述范围，应停止对应工作单元，另立计划，不在本批追加兼容分支。

## 3. 当前证据与架构决定

| 事实 | 当前证据 | 对计划的约束 |
| --- | --- | --- |
| vendor 由 codeload/tarball 导入 | `third_party/codex-cli/UPSTREAM.md` | 不依赖根仓库 merge-base；用固定提交树做三方比较 |
| `UPSTREAM.md` 声称只有 1 个 active overlay | 当前文件内容与数百条本地改动不符 | 数量必须由 inventory 生成，禁止人工维护 |
| 已存在多批选择性回移 | `docs/migration/codex-sync/` 与提交 trailers | 建立 ledger，校验 upstream/local SHA 和 patch digest |
| `cargo test` 的一个 TUI 用例曾发生栈溢出 | 第一批执行记录 | 不把单进程 libtest 作为全量门禁入口 |
| 同一用例在 Nextest、现有 8 MiB 栈下通过 | 本计划编写阶段定向验证 | 优先采用每测试进程隔离；不预设提高 vendor 栈配置 |
| 当前 TUI 全量结果为 1843 pass、33 fail、1 ignored | 第一批 Linux 回归 | 首次基线必须如实记录 33 个失败，不得先改结果再建基线 |
| 快照测试可能生成 `.snap.new` | Insta 默认行为 | 门禁固定 `INSTA_UPDATE=no`，任何快照接受必须人工审阅 |

新同步工具全部放在 Whale 根目录 `scripts/codex-upstream/`；除修正文档事实外，不把同步逻辑写入 vendor。这样 vendor 后续替换时，校验工具和机器账本不会随快照被覆盖，也不会把自己的输出递归计入 overlay。

## 4. 事实源与数据流

```text
固定上游基线 Git tree ─┐
                        ├─ overlay generator ─ overlay-inventory.json
当前 vendor Git index ──┘                         │
                                                  ├─ sync metadata validator
提交历史 + migration logs ─ backport-ledger.json ┘

codex-tui tests ─ Nextest/JUnit ─ normalized result ─ tui-baseline.json
Windows tests/smoke ─ evidence record ──────────────── execution log
```

计划新增或维护的工件：

| 工件 | 位置 | 权威内容 | 状态 |
| --- | --- | --- | --- |
| 上游说明 | `third_party/codex-cli/UPSTREAM.md` | 导入来源、固定基线、当前目标、账本链接 | verified |
| overlay 清单 | `docs/v0.0.5/codex-upstream-sync/overlay-inventory.json` | vendor 相对固定基线的确定性路径差异 | verified |
| backport 账本 | `docs/v0.0.5/codex-upstream-sync/backport-ledger.json` | 已回移 upstream/local 提交映射 | verified |
| provenance backlog | `docs/v0.0.5/codex-upstream-sync/backport-provenance-backlog.json` | 有候选来源但缺少权威证据的本地回移 | verified |
| TUI 基线 | `docs/v0.0.5/codex-upstream-sync/tui-baseline.json` | 规范化测试集合与失败分类 | verified-current-fact；non-green |
| metadata 同步工具 | `scripts/codex-upstream/` | inventory 生成、合同和统一校验 | verified |
| TUI runner | `scripts/codex-upstream/` | Nextest/JUnit 执行与规范化 | verified |
| Windows runner | `scripts/codex-upstream/` | Windows 自动验证 | planned |
| 执行记录 | `docs/migration/codex-sync/2026-08-01-upstream-baseline-and-test-gates.md` | 命令、结果、例外、证据路径 | partial |

JSON 工件记录 schema version，但不写生成时间、绝对路径、测试耗时、随机 run id 等易变字段。所有数组按稳定键排序；同一代码状态重复生成必须逐字节一致。

## 5. 工件合同

### 5.1 Overlay inventory

根字段至少包含：

- `schema_version`；
- `vendor_path`；
- `baseline_commit`；
- `target_commit`；
- `entries`；
- 按状态与分类汇总的 `summary`。

每个 entry 至少包含：

- 相对 vendor 根的 UTF-8 路径；
- `added`、`modified` 或 `deleted` 状态；
- baseline/current SHA-256，缺失侧为 `null`；
- Git `numstat` 的新增/删除行数，二进制文件显式标记；
- 一个或多个分类；
- 可追溯的本地 evidence commit 列表。

实施中根据 731 条真实路径审计扩充了分类词表。高风险产品边界为：

- `brand_home`
- `provider_model`
- `wire_sse`
- `cache_observability`
- `taskspace_domain`
- `taskspace_host_hooks`
- `multi_agent`
- `web_tools`
- `upstream_backport`
- `build_release`
- `unclassified`

此外使用 `app_server_protocol`、`apply_patch`、`cli_surface`、`cloud_remote`、`configuration`、`instructions_skills`、`mcp`、`permission_safety`、`protocol_contract`、`provider_transport`、`sandbox_exec`、`session_context`、`tool_runtime`、`tui_experience`、`runtime_utilities`、`generated_artifact`、`developer_tooling`、`test_fixture`、`documentation` 等明确子系统标签，避免用 `build_release` 吞掉未知路径。权威枚举以 `scripts/codex-upstream/classification.py` 为准。

分类允许多标签，每条记录同时输出 `matched_rule_ids`。`unclassified` 是显式债务，并阻断 vendor refresh；工具不得通过默认归类隐藏未知路径。生成器比较固定基线 tree 与当前 Git index 中的 vendor subtree，固定 `--no-renames` 并使用 NUL 分隔的机器格式处理特殊文件名。控制元数据 `UPSTREAM.md` 明确列入 `excluded_control_paths`，避免 inventory 与包含自身未知提交 SHA 的 evidence 形成递归；未来 vendor 代码变更先独立提交，再在后续元数据提交生成 inventory。evidence commit 默认保留最近 20 条并记录完整数量及截断标志，避免公共文件产生无界工件。若本地缺少固定基线对象，工具应失败并打印精确 fetch 指令，不自动添加 remote 或扩大获取范围。

### 5.2 Backport ledger

每条记录至少包含：

- 40 位 `upstream_commit`；
- 40 位 `local_commit`；
- 原始 patch 的 SHA-256；
- 目标路径列表；
- `applied`、`reverted` 或 `superseded_by_vendor` 状态；
- focused/crate/platform 验证结果及证据文档；
- 可选的替代或回退提交。

校验器必须拒绝：重复 active upstream SHA、缺失提交、路径不存在、ledger 与 commit trailers 不一致、基线不一致，以及记录为通过但没有证据路径的验证项。vendor refresh 后不得删除旧记录，只能将其标记为 `superseded_by_vendor`。

### 5.3 TUI baseline

全量入口采用 `cargo nextest`，runner 固定：

- `INSTA_UPDATE=no`；
- vendor 现有 `RUST_MIN_STACK=8388608`；
- `--no-fail-fast`；
- 专用 `whale-baseline` profile；
- JUnit 输出到临时或 ignored 目录，再规范化为仓库 JSON。

专用 Nextest 配置放在 `scripts/codex-upstream/`，runner 解析为绝对路径后通过 tool config 加载，避免修改 vendor `.config/nextest.toml`。规范化结果只保留测试稳定标识、结果和失败分类，不保留耗时、时间戳、临时路径或原始 ANSI 输出。

失败分类限定为：

- `snapshot_review`：渲染结果变化，需要审阅 `.snap.new` 与产品预期；
- `functional_assertion`：普通断言失败；
- `environment`：栈、平台、依赖工具或资源边界；
- `flaky_candidate`：相同配置重复结果不一致；
- `unknown`：证据不足，阻断最终门禁。

首次运行必须建立“当前事实基线”，允许包含已知失败；第二阶段才逐项降到零。基线不是豁免清单，新增失败或失败名称漂移立即失败。最终准入要求零失败，除非另有经过明确决策并写入合同的 ignored 测试。

## 6. 工作单元

计划编写阶段使用 `planned`、`blocked-on-discovery`、`deferred`；实施后只有满足验收信号的工作单元才标记 `verified`。

| ID | 状态 | 工作与主要文件 | 验证与完成信号 | 依赖 |
| --- | --- | --- | --- | --- |
| W0 | verified | 冻结工具版本、基线/目标 SHA、当前失败列表；写执行记录表头 | Git 对象、Nextest/Insta 版本和初始命令可复现 | 无 |
| W1 | verified | 定义 inventory/ledger/baseline JSON schema 和分类规则 | schema fixture 的正反例单测通过 | W0 |
| W2 | verified | 实现 `generate_overlay_inventory.py`，生成首份 inventory | 连续生成逐字节相同；特殊路径 fixture 通过 | W1 |
| W3 | verified | 从历史文档与 trailers 整理 backport ledger | 每个 upstream/local SHA、digest、路径均校验通过 | W1 |
| W4 | verified | 实现 `validate_sync_metadata.py`，修正 `UPSTREAM.md` | 一条命令校验三份元数据且无 `unclassified` | W2、W3 |
| W5 | verified | 实现 Nextest/JUnit runner 与规范化 parser | 定向栈敏感用例在 8 MiB 下稳定通过；runner 不生成 `.snap.new` | W0、W1 |
| W6 | verified | 连续 3 次捕获当前 TUI 全量失败集合并分类 | 三次测试名集合一致；差异被标记为 flaky candidate | W5 |
| W7 | in-progress | 审阅并处理 status/model/detail 类快照；模型可见性部分已决策，status 细节仍待审阅 | 每份 diff 有产品判断；该组定向测试归零 | W6 |
| W8 | verified | 接受 chatwidget/guardian/MCP 中 Flash 默认与 Pro 隐藏的 20 个快照 | 514 个 chatwidget 测试通过；全量基线中该组失败归零 | W6 |
| W9 | blocked-on-discovery | 诊断 ActionMap route-mode 等功能断言 | 根因写入执行记录；修复有回归测试且不改变 TaskSpace 合同 | W6 |
| W10 | blocked-on-discovery | 隔离复现潜在 memory-setting flake | 同配置 3 次结果稳定，或找到可验证根因 | W6 |
| W11 | planned | 运行最终 TUI 全量门禁并固化零失败基线 | Nextest exit 0，规范化结果无失败/unknown | W7-W10 |
| W12 | planned | 增加 Windows PowerShell 自动验证入口 | mixed-case URL、shell-command crate、paste-burst 定向测试通过 | W0 |
| W13 | blocked-on-discovery | 在 VS Code 集成终端和 Windows 原生终端执行 paste smoke | 两类终端各有环境、步骤、结果证据；未执行则保持阻塞 | W12、Windows 环境 |
| W14 | planned | 收口执行文档、README 状态和所有账本 | 全部门禁通过；主题提交已 push；工作树干净 | W4、W11-W13 |

W7 和 W8 的每个快照族必须独立提交。若快照显示用户可见交互语义变化，先向用户说明旧/新行为及影响，得到决策后再接受；不得运行批量 `cargo insta accept` 代替审阅。

### 6.1 W8 模型可见性决策记录

- 决策日期：2026-08-01；
- 决策：`deepseek-v4-flash` 继续作为默认且当前唯一可见模型，`deepseek-v4-pro` 继续隐藏；
- 原因：DeepSeek 官方 Responses API 对 Pro 的适配仍在进行，不能把模型目录可用误写成当前 Codex Responses API 路径已经完成产品适配；
- 恢复门槛：官方适配上线，并通过 Whale provider 请求、reasoning/tool-call streaming、缓存回归、模型选择器与 TUI 全量测试；
- 验证：20 个已审阅 chatwidget 快照独立提交，514 个 chatwidget 测试通过；更新后的全量基线只剩 12 个 status 快照与 1 个 ActionMap 功能断言。

W9、W10、W13 当前信息不足，使用 `blocked-on-discovery` 是计划状态，不表示实施已被阻断。各工作单元在执行阶段先收集证据，再决定修复或另立需求。

## 7. 分阶段实施与提交边界

### Phase 1：同步事实源

按 W0 → W1 → W2 → W3 → W4 执行。建议提交序列：

1. `test(upstream-sync): define metadata contracts`
2. `feat(upstream-sync): generate deterministic overlay inventory`
3. `docs(upstream-sync): record selective backport ledger`
4. `feat(upstream-sync): validate vendor metadata`

Phase 1 退出条件：inventory 可复现、ledger 无重复、`UPSTREAM.md` 与机器账本一致、所有 overlay 路径已分类。未满足前不得开始 vendor refresh。

### Phase 2：恢复 TUI 门禁

按 W5 → W6 → W7/W8/W9/W10 → W11 执行。W7-W10 可在 W6 后独立推进，但不得并行修改同一快照或测试文件。每个快照族或根因修复独立提交，提交消息不得混写“更新快照”和“修复功能”。

Phase 2 退出条件：全量 Nextest 零失败、没有 `.snap.new` 遗留、三次重复集合稳定、基线 JSON 与当前结果一致。

### Phase 3：Windows 与批次收口

按 W12 → W13 → W14 执行。自动测试和人工 smoke 分开记账；没有真实 Windows 结果时允许合并脚本，但第二批整体状态保持未验证。

Phase 3 退出条件：Windows 自动测试通过，两类终端 paste smoke 有证据，执行记录和专题首页同步更新，所有改动已提交并推送。

## 8. 验证矩阵

以下命令是计划接口；脚本实现时保持名称和退出码语义稳定。

### 8.1 Metadata 工具

```bash
python3 -m unittest discover \
  -s scripts/codex-upstream/tests \
  -p 'test_*.py'

python3 scripts/codex-upstream/generate_overlay_inventory.py --check
python3 scripts/codex-upstream/validate_sync_metadata.py
```

`--check` 只比较生成结果，不改文件。普通生成模式只允许更新目标 inventory，不能改 Git remote、fetch 或提交状态。

### 8.2 TUI

```bash
python3 scripts/codex-upstream/run_tui_baseline.py --check

cd third_party/codex-cli/codex-rs
INSTA_UPDATE=no RUST_MIN_STACK=8388608 \
  cargo nextest run -p codex-tui \
  --profile whale-baseline \
  --no-fail-fast
```

快照审阅期间使用 focused test 和 `cargo insta review`；先看 diff，再决定保留代码还是更新快照。最终检查：

```bash
git status --short
find third_party/codex-cli/codex-rs -name '*.snap.new' -print
```

第二条命令必须无输出。

### 8.3 Windows

```powershell
cargo test -p codex-shell-command powershell_start_process_mixed_case_urls_are_dangerous --lib
cargo test -p codex-shell-command --lib
cargo test -p codex-tui paste_burst --lib
powershell -File scripts/codex-upstream/verify-quick-backports.ps1
```

人工 smoke 分别在 VS Code integrated terminal 与 Windows Terminal 记录：终端版本、粘贴内容规模、事件是否被错误拆分、最终输入内容和日志位置。不得把 Wine、Linux PTY 或编译通过替代真实 Windows 交互证据。

### 8.4 仓库门禁

```bash
git diff --check
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
git status --short --branch
```

本批原则上不应触及缓存敏感面。若 gate 报告敏感路径，必须按仓库预算规则停止并说明原因，不得用 `--no-verify`。

## 9. 风险、停止条件与回退

| 风险 | 预防与停止条件 | 回退方式 |
| --- | --- | --- |
| inventory 把 backport 当 Whale 自有设计 | ledger 先于分类验收，允许多标签 | 回退分类规则和生成工件，不改 vendor |
| Git 对象在其他环境缺失 | 明确报缺失 SHA 与精确 fetch 指令 | 获取单个对象后重跑，不新增长期 remote |
| 测试结果含路径/时序噪声 | JUnit 规范化时丢弃易变字段 | 删除临时报告并重生成基线 |
| 快照批量接受掩盖回归 | `INSTA_UPDATE=no` 默认；逐族人工 review | 单独 revert 对应快照提交 |
| ActionMap 修复改变 TaskSpace 合同 | 先定位根因，运行 TaskSpace 定向回归 | revert 独立功能提交；另立架构计划 |
| 8 MiB 下仍有稳定栈溢出 | 记录具体测试与 backtrace 后停止 | 先修深递归/大栈对象；提高栈仅作为有证据的后续决策 |
| Windows 环境不可用 | 保持 W13 为阻塞，不伪造证据 | 可先合并可审阅脚本，不关闭批次 |

任一工作单元出现以下情况时停止扩展：触及 DeepSeek final wire、TaskSpace canonical state、cache prefix、协议 schema、vendor 大面积替换，或需要真实 Whale Agent run。此类变化必须另做需求确认和工程计划。

每个主题提交均可用 `git revert <sha>` 独立回退；禁止通过重置分支或删除历史回退。JSON 工件与生成器必须在同一主题提交内保持一致。

## 10. 验收清单

- [ ] `UPSTREAM.md` 不再包含人工维护且失真的 overlay 数量；
- [ ] overlay inventory 对同一 tree 连续生成逐字节一致；
- [ ] 所有路径已分类，没有 `unclassified`；
- [ ] backport ledger 覆盖历史记录和第一批六个提交，无 active 重复；
- [ ] metadata validator 对篡改 SHA、digest、路径和重复记录的 fixture 均会失败；
- [ ] TUI runner 不写 `.snap.new`，规范化结果无时间与路径噪声；
- [ ] 当前失败集合已经稳定捕获并分类；
- [ ] 所有快照均经过逐组审阅；
- [ ] TUI 全量 Nextest 最终零失败；
- [ ] Windows 自动测试和两类终端人工 smoke 均有真实证据；
- [ ] 缓存 index gate 通过且未启动真实 Whale Agent run；
- [ ] 执行记录、专题首页与机器工件一致；
- [ ] 每个小主题均已 commit、push，工作树干净。

## 11. 外部依据

- [Git diff options](https://git-scm.com/docs/diff-options)：`--name-status`、`--numstat` 与 `-z` 提供适合脚本消费、可安全处理特殊路径的差异输出；
- [Nextest JUnit support](https://nexte.st/docs/machine-readable/junit/)：JUnit 是 Nextest 正式支持的测试运行机器结果格式，可保存失败输出；
- [Nextest configuration](https://nexte.st/docs/configuring-nextest/)：仓库配置与 tool-specific 配置可以分层，适合保留 vendor 原配置并叠加 Whale 门禁；
- [Insta quickstart](https://insta.rs/docs/quickstart/)：`INSTA_UPDATE=no` 可令快照不匹配直接失败，避免测试过程自动更新快照；
- [Rust `std::thread`](https://doc.rust-lang.org/std/thread/index.html)：`RUST_MIN_STACK` 只控制新线程的最小栈，不能替代对递归或大栈对象根因的诊断。

这些依据决定了本批使用“确定性 Git 差异 + 机器账本 + Nextest/JUnit + 显式快照审阅”的门禁组合，而不是继续依赖人工计数、单次测试输出或批量接受快照。
