# 第三批：Codex 0.146 候选基底资格审查与 Overlay 重放工程计划

- 文档状态：实施中（W1 合同已完成）
- 计划模式：Execution Tracking
- 创建日期：2026-08-02
- 适用版本：WhaleCode v0.0.5
- 计划基线：`b460ece4e25bb0a7b6484ffe83b84a09ae46804d`
- 当前 vendor 固定基线：`fed0a8f4faa58db3138488cca77628c1d54a2cd8`
- 官方候选目标：Codex CLI `rust-v0.146.0` / `e363b08c9175ac1cbe5893615dd2cb9ddf95043b`
- 来源分析：[总体差异与合并策略](README.md)
- 前置结果：[第二批最终实施报告](../../migration/codex-sync/2026-08-02-upstream-baseline-and-test-gates-closeout.md)

## 1. 问题与目标

当前 Whale vendor 与官方 `rust-v0.146.0` 之间不适合直接 merge 或整目录覆盖：

- Codex 以 tarball/vendor 方式导入，Whale 根仓库与官方历史没有可用于普通 merge 的共同提交链；
- 固定基线到 0.146 有 2,790 个官方提交、4,209 个变化文件；
- 当前 Whale overlay inventory 有 730 个产品/代码路径，且 DeepSeek、TaskSpace、app-server、protocol、context、tool runtime 和 TUI 存在交叉修改；
- 上游已拆分 message history、prompts、context fragments、HTTP transport 和 app-server transport，孤立 cherry-pick 会携带跨 crate 依赖；
- 第二批已经建立事实基线和测试门禁，但尚无“0.146 原生候选是否可构建”和“每个 Whale overlay 如何重放”的机器合同。

第三批的目标不是替换 vendor，而是把后续 cutover 从一次不可审计的大改，变成一组有明确输入、disposition、依赖、验证和回滚边界的小批次。

完成后必须能够回答：

1. `e363b08c` 的官方源码在当前 Linux/toolchain 环境中能否独立解析、构建和运行声明的无模型测试；
2. 当前 730 个 Whale overlay 路径中的每一个应当采用、原样重放、语义适配、重新生成、删除还是延期；
3. generated artifacts 的权威生成源和命令是什么；
4. brand/home、通用 substrate、DeepSeek、TaskSpace/Multi-Agent 应按什么依赖顺序进入后续 cutover；
5. 哪些未决项会阻断真实 vendor 替换。

## 2. 范围与非目标

### 2.1 本批包含

- 冻结并核验 0.146 tag、commit、tree、发布日期和 license；
- 在仓库外临时目录导出纯官方候选，不创建 `third_party/codex-cli-next/`；
- 对纯官方候选执行可重复的无模型 build/test qualification；
- 建立 baseline → 0.146 的 upstream delta inventory；
- 建立 730 路径的 overlay replay ledger 和 schema；
- 建立 generated artifact → generator → command 的 lineage 清单；
- 建立后续 cutover 批次依赖图、门禁和安全停止条件；
- 形成第三批执行报告与 go/no-go 结论。

### 2.2 明确不包含

- 修改或替换 `third_party/codex-cli/` 当前生产 vendor；
- 新建或切换 Git 分支；
- 提交一份完整的 0.146 候选源码副本；
- 修改 DeepSeek provider、reasoning、usage、Responses wire 或缓存前缀；
- 修改 TaskSpace canonical map、event store、projection、host hooks 或 W9；
- 迁移 permission profiles、message history、HTTP/app-server transport 或 thread store；
- 启用 Plugins、Apps、MCP 2026、remote Code Mode、audio/image/realtime 等产品能力；
- 真实 Whale Agent run、provider probe 或任何模型费用；
- Windows 自动或人工验证。

上述内容只能成为 ledger disposition 或后续批次输入，不能在本批顺手实施。

## 3. 最小建设方案

本批复用第二批已有的 Git snapshot、metadata contract、overlay inventory 和 validator，不建立第二套同步框架。

```text
官方 tag/commit/tree
        │
        ├─ 临时导出 + pristine qualification ─ upstream-candidate.json
        │
固定 vendor baseline ──┐
                       ├─ upstream delta ─ upstream-delta-inventory.json
0.146 candidate tree ──┘

现有 overlay-inventory.json
        + upstream delta
        + generated lineage
        └─ overlay-replay-ledger.json
                 └─ cutover batch/dependency decision
```

最小新增面：

- 在 `scripts/codex-upstream/` 复用现有模块，新增窄用途 candidate/replay 命令；
- 在现有 `schemas/` 增加候选、delta、replay 三个 JSON schema；
- 在当前专题目录存放三个确定性 JSON 工件；
- 不新增 daemon、数据库、网络服务、运行时 feature、配置开关或生产依赖。

不采用以下更重方案：

- 不在仓库内保留 `codex-cli-next` 双 vendor；它会引入大规模重复源码、双锁文件和长期清理成本；
- 不为一次迁移新增通用插件/策略框架；当前需要的是可审计账本，不是运行时扩展系统；
- 不自动推断语义 disposition；规则只能填充可证明事实，语义选择必须有证据与责任边界。

## 4. 计划工件与合同

| 工件 | 类型 | 位置 | 权威内容 | 初始状态 |
| --- | --- | --- | --- | --- |
| candidate schema | contract | `scripts/codex-upstream/schemas/upstream-candidate.schema.json` | tag/commit/tree/license/toolchain/qualification 结构 | planned |
| delta schema | contract | `scripts/codex-upstream/schemas/upstream-delta-inventory.schema.json` | baseline → target 文件、crate、生成物与依赖变化 | planned |
| replay schema | contract | `scripts/codex-upstream/schemas/overlay-replay-ledger.schema.json` | Whale overlay disposition、证据、批次、阻塞关系 | planned |
| candidate command | tooling | `scripts/codex-upstream/qualify_candidate.py` | 导出、manifest、命令执行与结果规范化 | planned |
| replay command | tooling | `scripts/codex-upstream/generate_replay_ledger.py` | 合并现有 inventory/delta/lineage，生成确定性账本 | planned |
| candidate manifest | evidence | `docs/v0.0.5/codex-upstream-sync/upstream-candidate.json` | 0.146 不可变身份与 qualification 结果 | planned |
| delta inventory | evidence | `docs/v0.0.5/codex-upstream-sync/upstream-delta-inventory.json` | 官方结构变化事实 | planned |
| replay ledger | decision ledger | `docs/v0.0.5/codex-upstream-sync/overlay-replay-ledger.json` | 730 路径完整 disposition | planned |
| execution report | result | `docs/migration/codex-sync/<date>-rust-0.146-candidate-qualification.md` | 命令、证据、阻塞项和 go/no-go | planned |

### 4.1 Candidate manifest 必填字段

- `schema_version`；
- `release_tag`、`commit_sha`、`tree_sha`、`release_date`；
- `license_path`、`license_sha256`；
- `source_method`、`source_object_verified`；
- Rust/Cargo/Nextest 版本；
- 每条 qualification command 的 cwd、exit code、规范化结果和日志证据路径；
- `production_vendor_unchanged`；
- `model_request_count = 0`。

不得写入绝对临时路径、生成时间、用户名、API key、环境秘密或无必要的完整环境变量。

### 4.2 Replay disposition

| Disposition | 使用条件 | 禁止条件 |
| --- | --- | --- |
| `adopt-upstream` | Whale 当前语义已无保留必要，且有明确删除依据 | 仅因上游文件“更新”就覆盖 Whale 行为 |
| `reapply-exact` | Whale patch 可在目标树 clean apply，且目标输出 digest 可验证 | patch 依赖旧 crate/type 或命中生成物 |
| `adapt-semantically` | Whale 产品合同仍有效，但宿主接口已变化 | 未写明新旧控制流和验证方式 |
| `regenerate` | 已确认权威 generator、输入和命令 | 手工编辑 JSON/TS/schema/snapshot 冒充生成 |
| `drop` | 功能明确废弃、被上游等价能力替代或属于错误历史 | 没有产品/架构依据的删除 |
| `defer` | 已知归属后续专项且当前不阻断候选资格审查 | 用 defer 隐藏 owner、恢复条件或 cutover blocker |
| `blocked-on-discovery` | 当前缺少可证实来源、owner、生成链或语义合同 | 作为第三批退出时的最终状态 |

每条 ledger 至少包含：`path`、上游/Whale 状态、分类标签、disposition、依据、后续批次、验证、依赖、owner domain 和 generated lineage。第三批退出时不得保留 `blocked-on-discovery`。

## 5. 工作单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W0 | 冻结候选身份 | compatibility | `UPSTREAM.md`、candidate manifest | 0.146 tag/commit/tree/license | 从本地官方对象和官方 release 交叉核验并写入不可变字段 | 本批所有工具只接受一个目标身份 | 防止分析期间 latest 漂移导致账本混用不同版本 | Complexity: 增加一份 manifest 合同；Reach/Cost: 同步工具和文档校验需共同维护目标 SHA | `git cat-file`、`git rev-parse <sha>^{tree}`、license digest、官方 release 对照 | 任一身份不一致即停止，不生成后续工件 | planned |
| W1 | 定义机器合同 | internal | `schemas/`、`metadata_contract.py` | candidate/delta/replay schemas | 增加三个 schema 及正反例 fixture，扩展现有联合 validator | 缺字段、非法 disposition、重复路径、绝对路径、循环批次依赖和非确定字段会失败 | 让资格结果和迁移决策可由 CI 审计，减少人工表格漂移 | Complexity: +3 schema 和 validator 分支；Reach/Cost: Python test/维护范围扩大，不影响生产二进制 | 23 项 Python 单测与联合 validator 已通过 | 单提交 revert | completed |
| W2 | 导出纯上游候选 | developer-tooling | `qualify_candidate.py`、临时目录 | `git archive` candidate tree | 使用精确 commit 导出到 `mktemp` 目录，校验 tree/license 后打印可恢复证据路径 | qualification 不读取或覆盖当前 vendor | 排除 Whale 工作树污染，也避免提交第二份 vendor | Complexity: +1 临时导出流程；Reach/Cost: 使用额外磁盘和 IO，临时目录需在记录证据后安全清理 | 导出文件清单/tree identity 与 commit 一致；`git diff --exit-code HEAD -- third_party/codex-cli` | 导出不一致或触碰 vendor 立即停止；临时目录移入系统临时回收/安全清理 | planned |
| W3 | 验证 pristine substrate | compatibility | 临时候选 `codex-rs/`、candidate manifest | Cargo workspace 与关键 crates | 按官方工具链运行 fmt/check，并运行 CLI/core/app-server/TUI 的无模型测试集合，规范化 exit/result | 候选自身问题与 Whale replay 问题分离 | 在投入 overlay 迁移前暴露工具链、平台或上游基线 blocker | Complexity: 不增生产代码，增加 qualification command matrix；Reach/Cost: Rust 构建时间、磁盘和 CI 时间上升，无 API 费用 | `cargo fmt --all -- --check`；locked CLI check；关键 crate tests；结果写 manifest | 任一失败先分类 upstream/environment；禁止用 Whale patch 修候选以求通过 | planned |
| W4 | 建立 upstream delta | observability | `git_snapshot.py`、新 delta 生成逻辑 | baseline → target path/crate graph | 复用 no-renames 和确定性 JSON 规则，记录状态、hash、numstat、crate ownership、生成物标记 | 官方 4,209 文件变化可按结构和依赖查询 | 为后续按 crate/宿主边界切批提供事实，而不是按提交标题猜测 | Complexity: 增加 delta 生成与 crate mapping；Reach/Cost: Git 对象扫描时间增加，输出文件需版本化 | 连续生成 SHA-256 一致；路径总数与 `git diff --name-status` 一致；0 unknown ownership 或显式例外 | 计数/哈希不一致时停止，不进入 replay 分类 | planned |
| W5 | 建立 generated lineage | cleanup | delta/replay generator、Cargo/just/schema export 配置 | generated artifact 标签路径 | 为 protocol JSON/TS、config schema、snapshots 等记录 generator、输入和命令；不能证明的标为 discovery blocker | 后续 cutover 对生成物执行再生成而非手工三方合并 | 减少 schema 漂移和大批伪冲突，保留官方生成链 | Complexity: 增加 lineage 字段和识别规则；Reach/Cost: protocol/app-server/config 测试范围扩大，不改变运行时 | generator 路径存在；命令 `--check` 或 dry-run 输出稳定；人工抽查各生成族 | 任一生成族无权威来源则阻断对应 cutover 批次 | planned |
| W6 | 生成 replay ledger | compatibility | `generate_replay_ledger.py`、replay JSON | 现有 730 overlay paths | 合并 overlay/delta/lineage，自动填充事实字段；逐域审阅并填写 disposition、批次、验证和依赖 | 每个 Whale overlay 都有唯一且可追溯的处理方式 | 避免整仓替换时漏掉 DeepSeek、WHALE_HOME、缓存或 TaskSpace 合同 | Complexity: +1 决策账本及分类维护；Reach/Cost: 需要跨域人工审阅，后续 target 更新必须重算 | 730/730 唯一路径；0 非法/空 disposition；validator passed；抽查高风险标签 | 出现未归属/矛盾 disposition 即保留 blocked-on-discovery，不进入 W7 go 决策 | blocked-on-discovery |
| W7 | 划分 cutover 依赖 | internal | replay ledger、执行报告 | `cutover_batch` 与 `depends_on` | 按 brand/home → substrate → DeepSeek/wire → TaskSpace/Multi-Agent → generated/release 建立 DAG，单列跨域路径 | 后续每批都有明确输入、停止点和回滚边界 | 限制单次变更爆炸半径，避免两个状态权威或双 transport 长期并存 | Complexity: 增加迁移 DAG，不增加运行时分支；Reach/Cost: 后续提交/测试数量增加但归因更清晰 | DAG 无环；每条 adapt/regenerate 有批次和 verification；高风险路径有 owner domain | 发现循环依赖时不得用临时兼容分支掩盖，返回 W6 重分边界 | blocked-on-discovery |
| W8 | 完成资格结论 | documentation | 专题 README、执行报告、三份 JSON | go/no-go decision | 汇总 qualification、ledger completeness、blockers、后续批次及恢复条件，运行全套 validator 后提交推送 | 形成是否启动真实 vendor cutover 的可审计决策包 | 下一批可以按证据选定范围，不重复做全量差异调查 | Complexity: 增加一次性结果文档；Reach/Cost: 文档/账本维护和 review 成本，无运行时成本 | metadata tests、三份 `--check`、cache index gate、vendor diff clean、Git clean/push | 任一 blocker 未关闭则结论只能 no-go；不得以文档完成替代资格通过 | planned |

## 6. 分阶段执行

### Phase 1：不可变输入与候选自身资格

- Entry condition：当前分支与远端一致；第二批已收口；本地存在 `fed0a8f4` 和 `e363b08c` Git 对象。
- Work units：W0–W3。
- Phase-local evidence：target/tree/license manifest、schema tests、pristine fmt/check/test 结果、当前 vendor 零 diff。
- Cross-unit side effects：仅增加同步工具、JSON 工件和本地构建成本；不增加生产代码或运行时状态。
- Next-phase condition：候选身份确定；qualification 每项已 passed 或被明确分类为不影响 ledger 的环境 blocker。

### Phase 2：差异与重放决策

- Entry condition：Phase 1 工件通过 schema 和确定性检查。
- Work units：W4–W7。
- Phase-local evidence：delta inventory、generated lineage、730 路径 replay ledger、无环 cutover DAG。
- Cross-unit side effects：引入持续维护的迁移账本；若官方目标版本变化，三个工件必须整体重算，禁止混用。
- Next-phase condition：0 `blocked-on-discovery`、0 重复路径、0 无生成源的 `regenerate`、0 无验证的 `adapt-semantically`。

### Phase 3：资格结论与收口

- Entry condition：Phase 2 退出条件满足，或阻塞项足以形成明确 no-go。
- Work units：W8。
- Phase-local evidence：执行报告、validator、cache index gate、当前 vendor clean、提交与 push。
- Cross-unit side effects：只产生审计和后续计划义务，不授权任何生产 cutover。
- Next-phase condition：用户基于 go/no-go 报告决定是否建立第四批 brand/home cutover 计划。

## 7. 精确验证矩阵

### 7.1 仓库内合同

```bash
python3 -m unittest discover \
  -s scripts/codex-upstream/tests \
  -p 'test_*.py'

python3 scripts/codex-upstream/qualify_candidate.py --check
python3 scripts/codex-upstream/generate_overlay_inventory.py --check
python3 scripts/codex-upstream/generate_replay_ledger.py --check
python3 scripts/codex-upstream/validate_sync_metadata.py
```

### 7.2 候选身份与 vendor 隔离

```bash
git cat-file -e e363b08c9175ac1cbe5893615dd2cb9ddf95043b^{commit}
git rev-parse e363b08c9175ac1cbe5893615dd2cb9ddf95043b^{tree}
git diff --exit-code HEAD -- third_party/codex-cli
```

候选导出命令必须使用显式 commit 和经验证的临时路径，禁止以 `main`、`latest` 或未解析环境变量作为输入。

### 7.3 Pristine qualification

候选临时树中的实际命令由 W3 manifest 固化，最低包含：

```bash
cargo fmt --all -- --check
cargo check -p codex-cli --bin codex --locked
cargo nextest run -p codex-core
cargo nextest run -p codex-app-server
cargo nextest run -p codex-tui
```

若上游 workspace 对命令入口有调整，以 0.146 仓库内官方 README/justfile 为准，但必须在 manifest 中记录替换原因；不得修改候选源码让测试变绿。

### 7.4 Whale 仓库门禁

```bash
git diff --check
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
git status --short --branch
```

本批不触及缓存敏感生产路径。若 index gate 识别出敏感面，必须停止并说明具体路径；不得申请或启动真实回归来扩大本批范围。

## 8. 风险与停止条件

| 风险 | 触发信号 | 处理 | Safe Stop / Fallback |
| --- | --- | --- | --- |
| 目标版本漂移 | 官方发布新 stable | 本批仍固定 0.146；新版本另做 target 决策 | 不静默改 SHA，不混合工件 |
| 候选自身不通过 | pristine check/test fail | 区分 upstream、toolchain、environment，保存最小证据 | 结论 no-go 或带环境 blocker，不修改 vendor |
| 双 vendor 污染 | 仓库出现候选源码副本或锁文件 | 立即停止，移入安全备份并恢复仓库 clean | 保留 manifest，不保留第二源码树 |
| 自动分类掩盖语义 | 高风险路径被无证据标为 exact/adopt | validator 拒绝缺 evidence/verification 的 disposition | 返回 W6 人工审阅 |
| generated lineage 缺失 | `regenerate` 无 generator/command | 阻断对应 cutover batch | 暂标 blocked-on-discovery，不手工合并 |
| 依赖图成环 | batch DAG cycle | 重新划分宿主 seam，不加静默 fallback | no-go，另立架构决策 |
| 缓存敏感面扩张 | cache index gate 命中生产路径 | 停止第三批，报告路径与前缀风险 | 不用 `--no-verify`，不启动真实 run |
| 隐私或凭证泄漏 | manifest/log 含 home、token、key、绝对路径 | schema/normalizer fail closed | 不提交工件，清理敏感输出后重跑 |

## 9. 后续 cutover 批次草案

以下只是第三批输出要验证的依赖方向，不授权实施：

| 后续批次 | 初始边界 | 主要验证 | 当前状态 |
| --- | --- | --- | --- |
| 第四批 | Whale brand、`WHALE_HOME`、CLI/build/release overlay | home/auth 隔离、CLI smoke、package identity | deferred |
| 第五批 | message history、prompts、context fragments、HTTP/app-server transport、state substrate | upstream crate tests、resume/fork、protocol compatibility | deferred |
| 第六批 | DeepSeek Responses provider、model catalog、reasoning/usage、wire/cache | provider contract、SSE、free final-wire、cache gate | deferred |
| 第七批 | TaskSpace domain/host hooks、Multi-Agent/world-state 权威关系 | state/store/replay/fork/terminal/TUI viewer | deferred |
| 最终切换 | generated artifacts、workspace/CLI/release | 全量回归、schema generation、rollback rehearsal | deferred |

W7 必须验证这一顺序是否无环。若 brand/home 与 substrate 无法独立验证，第四、第五批应重分，而不是通过临时双路径强行维持。

## 10. 提交与审查边界

建议每个小主题独立提交并立即 push：

1. `test(upstream-sync): define candidate qualification contracts`
2. `feat(upstream-sync): qualify pristine 0.146 candidate`
3. `feat(upstream-sync): inventory upstream target delta`
4. `feat(upstream-sync): map generated artifact lineage`
5. `docs(upstream-sync): establish overlay replay ledger`
6. `docs(upstream-sync): record 0.146 cutover decision`

每次代码变更完成后按仓库规则询问是否执行对抗性审查。JSON 生成器和对应工件必须在同一提交中保持一致；不得创建新分支；不得把多个独立 disposition 域混入一个无法回滚的提交。

## 11. 验收清单

- [ ] 0.146 tag、commit、tree、release 和 license 身份一致；
- [ ] pristine candidate 未进入仓库且当前 vendor 零改动；
- [ ] candidate/delta/replay schema 正反例测试通过；
- [ ] pristine fmt/check/test 结果完整记录且模型请求为 0；
- [ ] upstream delta 路径计数、hash 和 crate ownership 可重复；
- [ ] generated artifact 全部存在 generator lineage 或显式 blocker；
- [ ] 730 个 Whale overlay 路径全部有唯一 disposition；
- [ ] `adapt-semantically`、`regenerate`、`defer` 均有验证和恢复条件；
- [ ] cutover DAG 无环，跨域路径有 owner domain；
- [ ] cache index gate 通过，未启动真实 Whale Agent run；
- [ ] 执行报告明确 go/no-go，不把账本完成等同于 vendor 已迁移；
- [ ] 所有改动已提交、push，工作树 clean。

## 12. 外部依据

1. [OpenAI Codex 0.146.0 Release](https://github.com/openai/codex/releases/tag/rust-v0.146.0)：固定本批候选版本、commit、发布时间和上游能力变化；
2. [OpenAI Codex 安装与源码构建文档](https://github.com/openai/codex/blob/main/docs/install.md)：确定 pristine candidate 的官方构建与测试入口；
3. [OpenAI Codex App Server 文档](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)：确认 app-server 是版本化协议/transport 表面，后续必须生成和兼容验证；
4. [OpenAI Codex 配置 Schema](https://github.com/openai/codex/blob/main/codex-rs/core/config.schema.json)：确认配置生成物应以官方 schema 和生成流程为事实源；
5. [Git `archive` 文档](https://git-scm.com/docs/git-archive)：候选导出使用精确 tree，不依赖工作树复制。

## 13. 本地证据索引

- `third_party/codex-cli/UPSTREAM.md`；
- `docs/v0.0.5/codex-upstream-sync/overlay-inventory.json`；
- `docs/v0.0.5/codex-upstream-sync/backport-ledger.json`；
- `docs/v0.0.5/codex-upstream-sync/tui-baseline.json`；
- `docs/migration/codex-sync/2026-08-02-upstream-baseline-and-test-gates-closeout.md`；
- `benchmarks/cache-regression/cache-surface-contract.json`；
- `coe/2026-08-01-23-55-w9-taskspace-mode-routing.md`。

第三批完成只代表“0.146 候选及 Whale overlay 已具备可审计迁移输入”，不代表 vendor 已升级，也不自动授权第四批。
