# WhaleCode v0.0.7 Codex CLI 0.151 主线追赶计划

- Status: complete（W1–W6 verified；聚焦收口复审通过）
- Plan Validity: valid-with-qualifications
- Created: 2026-08-31
- Product Authority: [`prd/2026-08-23-v0.0.6-multi-provider.md#confirmed-product-decisions`](../../../../prd/2026-08-23-v0.0.6-multi-provider.md#confirmed-product-decisions)
- Applicable Decisions: PD3、PD4、PD5、PD6、PD8、PD9、PD10、PD11、PD12、PD13、PD15、PD17、PD18、PD19
- Current vendor: `rust-v0.151.0` / `78c290807ce710180111df227df3b7a4fe845452`
- Target vendor: `rust-v0.151.0` / `78c290807ce710180111df227df3b7a4fe845452`
- Execution authorization: user-approved-execution-direct: 2026-09-01 “开始推进”

## 1. Execution Contract

- 上述 Product Authority 是本主题物质产品决策的唯一用户权威；修改、重释或替换 active 决策必须取得用户明确批准，Agent 不得自批。
- 已验证的代码、测试、上游源码和运行证据可以修订本计划，但不得静默改写 Product Authority。
- 新出现的用户可见产品选择必须延期、标记为 provisional，或交由用户确认；物质 `provisional` / `conflict` 会阻断依赖工作。
- 每个物质 Phase 结束后只审计该阶段的 Product Decision Delta；每个物质 Phase 开始前必须用已完成实现和证据 rebase 全部剩余计划。
- Pre-Phase Plan Rebase Gate 为 `pending` 或 `blocked-on-plan-approval` 时不得开始该 Phase。
- 设计方向、范围、模块/API/状态边界、工作单元、顺序、验证、回滚、收益、成本或风险发生物质变化时，必须记录 Plan Delta 并获得用户明确批准后才能继续。
- 纯工程范围、工作量、冲突数量或验证成本变化只需记录并由执行 Agent 治理，不要求用户批准；只有涉及产品逻辑、用户可见行为、状态权威或默认配置的 Plan Delta 才进入用户审批门。
- 不创建新分支；不触碰其他工作空间；不自动触发 GitHub Actions。真实模型运行仍受账本和预算门禁约束，本计划本身不授权任何付费请求。

## 2. Current And Expected Behavior

### 2.1 当前事实

- 当前分支已 rebase 并与 `origin/main` 对齐在 `a3ac0770df153dea2a18ff7e3cc5df245c76f45a`；v0.0.6 已发布。
- 当前 Codex production vendor 已切换至官方 0.151，并恢复 Whale identity、workspace isolation 及保证全树编译所需的既有 Provider/TaskSpace 接口 seam；Provider/DeepSeek 与 TaskSpace 的行为级验证仍分别归属 W4、W5。
- 官方最新稳定目标 0.151 的 peeled commit 为 `78c290807ce710180111df227df3b7a4fe845452`，tree 为 `68d61fd9886a749a78487d8ce950e3cb9309a3d7`；0.152 仍是 alpha，不在本主题范围。
- 0.149→0.151 官方差分为 1,660 个路径；v0.0.6 最终 Git index 相对 0.149 有 883 个 Whale overlay 路径，其中 306 个与上游差分相交。只读三方试应用发现 64 个明确冲突文件，另有 1 个 shell handler 无法直接套用、1 个 Whale 已删除的 macOS CLI 路径无法由 patch 原位更新。
- 冲突集中在 provider/model、ToolRouter、compaction、session/state、TaskSpace、extension lifecycle、app-server/TUI 与生成 schema；因此本主题不是“无冲突快速合入”。

### 2.2 期望结果

- vendor provenance 精确指向官方 0.151，不混入 0.152 alpha 或未固定的 main 提交。
- 吸收 0.150/0.151 的权限、安全、PTY shutdown、sandbox、unified-exec、shell snapshot、model-aware ToolRouter/MCP 修复。
- 保留 v0.0.6 三访问路由、凭据隔离、route/model 持久化、跨 Provider history projection 与命令可用性语义。
- 保留 DeepSeek 三模型目录、Flash 默认、Responses SSE、1M/755K、本地压缩和 final-wire/cache 合同。
- 保持 TaskSpace relational store 为唯一任务状态权威，fork/resume/reload 和 extension seam 不退化。
- 所有未验证平台或未启用上游产品面明确记录为延期，不以修改 Whale 默认值换取上游测试变绿。

## 3. Goals And Non-Goals

### Goals

1. 以不可变候选完成 0.151 pristine 资格验证和可复现 provenance。
2. 先导入上游 substrate，再按清晰 replay batch 恢复 Whale 产品 overlay，避免把机械迁移与业务改造混成一个工作单元。
3. 优先获得已确认的安全、稳定性和效率收益，并为受影响路径建立定向回归证据。
4. 最终以本地隔离回归、同步 metadata、schema、缓存 gate 和 clean/push 证据收口。

### Non-Goals

- 不追 0.152 alpha，不跟随未发布的 Codex main。
- 不因本次同步新增 Provider、改变默认模型或改写多 Provider 产品决策。
- 不自动启用 Bedrock、OpenAI remote plugin/catalog、ChatGPT 专属产品面或其他未确认默认值。
- 不借同步重构 TaskSpace、PrimitiveModule 或建立第二套状态权威。
- 不为上游专属测试全绿而扩大当前产品范围；Windows 专属验证另设平台单元。

## 4. Minimum-Sufficient Design

采用“固定候选 → 生成 replay 合同 → pristine substrate → 分域恢复 overlay → 发布收口”的最小路径：

1. `qualify_candidate.py` 只负责不可变 0.151 候选和隔离测试，不改变 production vendor。
2. delta/inventory/replay 工件先给出路径归属和恢复顺序，禁止凭冲突现场临时发明架构。
3. substrate cutover 以官方 tree 为机械输入；Whale 手写逻辑只在既有 seam 上恢复，不建立兼容层套兼容层。
4. Provider/DeepSeek 与 TaskSpace/Extension 分开闭环；生成 schema、snapshot 和 lockfile 由对应源代码单元生成，不手工拼接。
5. 缓存敏感面先走免费 index gate；只有门禁明确要求 revalidation 时，才另行申请真实运行预算。

## 5. External Evidence

- [Codex CLI 0.150.0](https://github.com/openai/codex/releases/tag/rust-v0.150.0)：权限、安全、PTY shutdown、unified-exec/TUI 效率以及 task/extension 基础变化。
- [Codex CLI 0.150.1](https://github.com/openai/codex/releases/tag/rust-v0.150.1)：retained image compaction budgeting 修复。
- [Codex CLI 0.151.0](https://github.com/openai/codex/releases/tag/rust-v0.151.0)：permission profile、sandbox、model-aware ToolRouter、MCP result、subagent usage 与 Guardian 修复。
- [0.149→0.151 官方比较](https://github.com/openai/codex/compare/rust-v0.149.0...rust-v0.151.0)：本主题固定的完整上游差分边界。

## 6. Pending Product Decisions

| ID | Decision Surface | Current / Proposed Behavior | Why Material | Evidence | Impact If Changed |
| --- | --- | --- | --- | --- | --- |
| PPD1 | Codex terminal task mentions/management 与 TaskSpace 的共存方式 | 当前只把上游能力纳入资格分析；若无法在不改变 TaskSpace 语义和命令入口的前提下共存，则延期该用户可见入口 | 同时暴露两种“task”概念会改变用户心智、TUI 路由和状态权威 | 0.150 引入 `@` task mentions 以及 read/create/message task 工具；当前冲突覆盖 app/thread routing 和 slash command | 若确认共存，需要定义命名、入口和状态边界；未确认时不阻塞其他安全/性能 substrate，只阻塞相关 UI 激活 |

## 7. Pre-Investment Validation

| ID | Critical Assumption | Decision Unlocked | Cheapest Credible Method | Enough Evidence / Not Proven | Budget / Isolation | Stop / Cleanup | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| V1 | 官方 0.151 在当前 Linux/toolchain/V8 合同下可作为稳定 substrate | 是否进入 cutover | 更新候选常量后导出 pristine tree，运行现有 fmt、CLI、code-mode-host、core/app-server/TUI 隔离资格矩阵 | 足够：固定 tag/commit/tree、依赖和主要 crate 结果可复验；不证明 Whale overlay 已兼容 | 0 模型请求；临时目录和隔离 home | 候选构建或基础测试出现方向性阻断即停止，production vendor 不变 | direction-supported |
| V2 | 当前 overlay 能按既有 seam 重放而不新建第二套架构 | W3–W5 的分域边界 | 生成 0.149→0.151 delta、0.151 overlay inventory/replay，复核全部交集和冲突归属 | 足够：306 个交集均有唯一 batch、验证和 safe stop；64 个三方冲突与 2 个非冲突式 apply failure 已单列；不证明运行时正确 | 纯 Git/脚本、0 请求 | 分类无法保持单一状态/route 权威时阻断对应 Phase | supported-with-expanded-scope |
| V3 | 0.151 的安全/效率收益没有被后续提交回退 | 是否需要额外 cherry-pick | 在固定 0.151 tree 中核对关键 commit 可达性和最终测试；不先 cherry-pick 到 production | 足够：目标 tree 包含 `035295b46e` sandbox/MCP/approval、`0182ff3480` model-aware ToolRouter、`bf3eb2ec91` PTY shutdown、`6677fd827d` / `528fd7ace5` retained-image budgeting、`5bf0ba3dd6` MCP result hook；不证明 Whale 合入后通过 | 只读源码和本地测试 | 缺失则从计划移除该收益，不从 alpha 补丁偷渡 | supported |

## 8. Work Units

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W1 | 固定并资格验证 0.151 候选 | qualification | `scripts/codex-upstream/qualify_candidate.py`、candidate metadata/evidence | tag/commit/tree、V8/toolchain、pristine test manifest | 将候选合同从 0.149 更新到固定 0.151，并在临时目录运行既有隔离矩阵 | 获得不触碰 production vendor 的 go/no-go 证据 | 在大规模 cutover 前排除无效目标 | Complexity：只改既有常量、fixture 和证据；Reach/cost：本地构建与长测试，0 模型请求 | tag/commit/tree/license、sync script tests、fmt、CLI、code-mode-host、core/app-server/TUI candidate logs | 任一方向性阻断即保留 0.149；W1 可独立 revert | verified |
| W2 | 生成可执行 replay 合同 | provenance/analysis | `scripts/codex-upstream/`、`docs/releases/v0.0.7/codex-upstream-sync/` | upstream delta、overlay inventory、replay batches、conflict ledger | 以 0.151 为目标生成路径级工件，并把冲突归入 generic、provider/DeepSeek、TaskSpace/extension、generated 四类 | 后续每个冲突只有一个 owner 和验证边界 | 降低现场误合并和跨域重构风险 | Complexity：metadata/文档，无运行时分支；Reach/cost：Git 分析、人工审阅，0 请求 | generator reproducibility、schema validator、path counts、三方 apply check | 分类不闭合则不进入 cutover；production vendor 保持 0.149 | verified |
| W3 | 导入 0.151 substrate 与通用 Whale seam | vendor/generic | `third_party/codex-cli/` 通用 build、identity、home、workspace、sandbox、exec/PTY 路径 | 官方 tree、Whale CLI/workspace seam、安全和效率修复 | 以官方 tree 机械替换 substrate，按 W2 replay 最小 identity/workspace seam；不在本单元设计 Provider 或 TaskSpace 新语义 | 通用底座升至 0.151，并获得权限、安全、shutdown 与执行效率修复 | 避免逐提交 cherry-pick 形成长期漂移 | Complexity：大规模机械 vendor 变化，上游文件豁免普通 500 行限制；Reach/cost：build/sandbox/exec/TUI 基础路径，0 请求 | conflict-free index、fmt/check、CLI identity、workspace doctor、权限/sandbox/PTY/unified-exec 定向测试 | 单独 commit；失败可 revert W3 回到完整 0.149，不在半冲突状态提交 | verified |
| W4 | 恢复 Multi-Provider 与 DeepSeek 合同 | provider/cache/context | login、model-provider-info、models-manager、core client/session/ToolRouter/compaction、TUI provider/model | route-bound auth、model catalog、prepared transition、history projection、DeepSeek Responses/final-wire | 在 0.151 seam 上重放 v0.0.6 provider overlay，吸收 model-aware ToolRouter/reasoning 修复；retained-image budgeting 只对支持的 route 生效，DeepSeek 保持本地压缩 | 三访问路由和 DeepSeek 三模型行为不退化，同时获得上游模型切换修复 | 直接保护当前主要产品链和缓存正确性 | Complexity：只改既有 route/capability seam，不新增 mega-registry；Reach/cost：凭据、模型、请求、压缩、cache，可能触发真实 revalidation 申请 | login/model/provider/core/TUI 定向测试、DeepSeek SSE、provider transition、history projection、Standard final-wire、`check_cache_regression_gate.py --source index` | 免费 gate 要求真实 revalidation 时立即停止并申请预算；W4 commit 可独立 revert 到 W3 substrate 状态 | verified |
| W5 | 恢复 TaskSpace、Extension 与 app-server/TUI 组合链 | taskspace/extension/protocol | state、tools、core extension lifecycle、app-server protocol/server、TUI routing/viewer、generated schema | relational store、fork/resume/reload、MCP result lifecycle、TaskSpace RPC/events | 在单一 relational state authority 上适配 0.151 session/extension 协议，生成 schema；PPD1 未确认前不激活冲突的 task UI | TaskSpace 与 extension 组合链在新 substrate 上可恢复、可重放 | 保留差异化能力并吸收 MCP result hook | Complexity：适配既有 extension seam，不建第二 store；Reach/cost：state/core/app-server/TUI/schema，0 请求 | state/tools/core TaskSpace、fork→reload→request、extension lifecycle/MCP result、app-server schema、TUI routing/viewer | 发现双状态权威或 PPD1 依赖即阻断相关入口；W5 独立 commit/revert | verified |
| W6 | 发布资格与证据收口 | release/metadata | provenance、replay、UPSTREAM、release report、cache evidence | 0.151 vendor identity、完整受控矩阵、延期边界 | 保持 cutover replay 工件不可变，生成 0.151 substrate 上的 current overlay inventory；运行当前 vendor 隔离回归与静态门禁，记录非绿项真实签名并提交推送 | v0.0.7 获得可审计的 0.151 substrate 候选 | 防止“版本已改但证据未跟上”，且不让发布后状态反向污染迁移历史 | Complexity：文档/metadata/generated，无新产品逻辑；Reach/cost：长时间本地测试和最小真实缓存资格 | historical replay structural check、current overlay reproducibility、schema、fmt、isolated affected/full matrix、cache gate、clean tree、remote sync | 未达到退出条件不宣称完成；真实验证必须有预算和账本 | verified |

## 9. Phases

### Phase A：候选资格与 replay 合同（W1–W2）

#### Pre-Phase Plan Rebase Gate

- Rebase scope: `main@a3ac0770d`、0.149 vendor、v0.0.6 已发布实现、固定 0.151 tag/commit/tree、当前 overlay/diff 证据
- Material plan delta: PDL3（历史 cutover replay 与 post-cutover current overlay 分离）
- Plan delta record: PDL3
- User approval: user-approved-execution-direct: 2026-09-01 “开始推进”
- Gate status: ready

- Entry: 工作区 ready、工作树 clean、固定 tag 可达。
- Exit: V1–V3 有证据结论；W1–W2 verified；production vendor 仍为 0.149。
- Product Decision Delta: 预期 `engineering-only`；不改变用户可见行为。
- W1 evidence（2026-09-01）：固定 0.151 commit/tree、license、Rust 1.95 与官方 `rusty_v8 150.4.0` 合同通过；fmt、CLI offline check、code-mode-host build 3/3 通过。pristine core 为 3808/3815、7 failed、9 skipped；其中 6 个失败与 0.149 同签名，新增 1 个 cyber access timeout。app-server 为 1384/1389、1 failed、4 zsh-fork timed out、2 skipped；唯一普通失败与 0.149 的 external-agent-config 签名相同。TUI 为 3955/3982、27 failed、6 skipped，27 个失败与 0.149 的宿主/快照集合一致。结论为 `direction-supported-with-known-test-risks`；production vendor index tree 未改变，模型请求 0。
- W2 evidence（2026-09-01）：生成并复验 883 路径 overlay、1,660 路径 upstream delta、883 条 replay decision 和覆盖 306 个交集的 conflict ledger；其中 240 个可三方 clean apply、64 个三方冲突、1 个 shell handler 硬套用失败、1 个 index 路径缺失。五个 batch 分别拥有 146/274/73/162/228 条 replay decision，所有路径均有唯一 owner、disposition 与 verification。全局 sync metadata validator 和 56 个脚本测试通过；production vendor 仍为 0.149，模型请求 0。

### Phase B：0.151 substrate cutover（W3）

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase A 证据、最终 replay batches、当前 production vendor
- Material plan delta: PDL1（replay 范围由旧清单 292/115 扩为真实 883/306）
- Plan delta record: PDL1
- User approval: not-required（engineering-only；用户于 2026-09-01 明确授权此类变化由 Agent 自行治理）
- Gate status: ready

- Entry: Phase A verified，方向为 supported，所有通用冲突已归属。
- Exit: vendor 指向 0.151；通用 build/identity/workspace/security/exec 定向验证通过；Provider/TaskSpace 未被误宣称完成。
- Product Decision Delta: none；未改变默认权限、已确认的用户可见能力或自动化行为。
- W3 evidence（2026-09-01）：production vendor 已机械切换至固定 `rust-v0.151.0` tree；恢复 Whale CLI identity、`WHALE_HOME`、workspace seam、隐藏 TaskSpace debug export，以及全树编译必需的 Provider credential / TaskSpace protocol 与 TUI 接口。上游新增 `0051_thread_artifacts.sql` 与已发布 Whale migration 版本碰撞，已将 TaskSpace migration 无损后移到 0052/0053，并用精确 checksum、原子降序改号和 fail-closed 校验兼容 0.149 DB。fmt、`cargo check -p codex-cli --tests`、state migration 17/17、CLI 主二进制 256/256、完整 CLI lib/integration targets、PTY 27/27 均通过；模型请求 0。旧 v0.0.5 overlay/replay metadata 相对 0.151 index 变旧是 W6 已规划的生成物刷新，不冒充 W3 运行时回归。Provider/DeepSeek 与 TaskSpace 仅恢复到编译/入口连续性，行为级退出条件仍归 W4/W5。

### Phase C：Multi-Provider 与 DeepSeek 重放（W4）

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase B 实际代码、0.151 provider/session/ToolRouter/compaction seam、剩余 W4–W6
- Material plan delta: PDL2（W3 编译闭包需要提前恢复少量 W4/W5 接口 seam，但不提前认领行为验证）
- Plan delta record: PDL2
- User approval: not-required（engineering-only；不改变产品权威或默认行为）
- Gate status: ready

- Entry: Phase B verified；现有 PD 可在 0.151 seam 上保持。
- Exit: provider/DeepSeek 定向矩阵与免费 cache gate 通过，或按门禁明确停在预算审批前。
- Product Decision Delta: 对照 Applicable Decisions 逐项审计 route、凭据、模型、history、compaction 和持久化。
- W4 evidence（2026-09-01）：恢复 DeepSeek Flash/Pro/Vision bundled catalog、Flash 默认和 1M/755K 合同；补齐 Bedrock access keys 的认证生命周期与首次外部认证失败后的 0.151 bounded recovery；TUI 初始 bootstrap 现消费 app-server provider groups。login 163/163、model-provider-info 30/30、models-manager 55/55、TUI provider 9/9、DeepSeek API 2/2，以及 core/app-server 的 route、history projection、provider transition、compaction 与 default-provider 定向矩阵均通过。聚合测试暴露的 test-only 0.151 接口迁移已以独立提交 `b393bca6be` 修复。cache gate 对 index 指纹 `a14e29c02a1c36c51815072a0c948137fab58e8d60da8a58b1b4fd246b739abb` 返回 PASS，Standard/TaskSpace final-wire 从不可比较恢复为可比较 candidate change；未修改既有 accepted baseline，发布晋升继续留在 W6，模型请求 0。Product Decision Delta 为 none：默认模型、三访问路由和 DeepSeek 本地压缩语义均未改变。

### Phase D：TaskSpace、Extension 与 TUI 组合链（W5）

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase C 实际代码、0.151 extension/app-server/TUI seam、PPD1 状态、剩余 W5–W6
- Material plan delta: none
- Plan delta record: not-required
- User approval: not-required（engineering-only；PPD1 相关用户入口保持隐藏，不在 W5 激活）
- Gate status: ready
- Rebase result: W4 已 verified；0.151 final-wire candidate 的真实基线晋升只影响 W6 发布资格。W3 已恢复大部分 W5 编译 seam，因此 W5 以当前实现的分层测试为事实基线，只修复 0.151 Extension API/协议兼容和真实组合链缺口，不机械覆盖整套旧 overlay，不建立第二状态权威。

- Entry: Phase C verified；TaskSpace 仍可保持单一 relational state authority。
- Exit: state/tools/core/app-server/TUI 组合链闭合；schema 可复现；PPD1 未决入口保持延期。
- Product Decision Delta: 审计任务概念、命令入口、extension tool result 顺序和持久化语义。
- W5 evidence（2026-09-01）：TaskSpace Extension 已迁移到 0.151 的 `for<'call> ToolExecutor<ToolCall<'call>>` 生命周期合同，test fixture 补齐 `ToolCallSource::Direct`；未改变工具执行顺序、relational store 或用户入口。state 210/210 与 doctest 1/1、extension-api 三组 3/3+8/8+5/5、TaskSpace Extension 41/41、core TaskSpace 75/75、response finalization 1/1、fork→reload 1/1、TUI TaskSpace 3/3 均通过。app-server protocol schema 按仓库生成流程刷新后全量 299/299 通过（1 ignored）；core MCP result 3/3、Extension/tool-search 18/18。Whale 默认 DeepSeek 导致上游 tool-search 测试隐含的 OpenAI namespace-tools 前提失效，已改为显式 OpenAI test fixture，生产 provider 能力和默认值未改。0.151 移除重复 `shell_command` wire tool 属于 `UnifiedExec` 归一化，配置别名与 `exec_command`/`write_stdin` 能力仍在，因此不恢复旧快照；PPD1 入口继续隐藏。模型请求 0，Product Decision Delta 为 none。

### Phase E：发布资格与收口（W6）

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase A–D 全部实现和证据、当前测试基线、剩余发布边界
- Material plan delta: none
- Plan delta record: not-required
- User approval: not-required（engineering-only；不修改产品权威、默认值或用户入口）
- Gate status: ready
- Rebase result: W1–W5 已形成独立提交并推送，当前 vendor、Provider/DeepSeek、TaskSpace/Extension 与 schema 定向矩阵均闭合。W6 只刷新现有 provenance/replay/UPSTREAM/release evidence，运行隔离回归和免费 cache gate；不再修改生产行为。若 cache gate 明确要求真实请求，则仅该预算门停止并向用户申请，不阻塞其余零成本收口工作。

- Entry: W1–W5 verified；无未解决的物质 conflict/provisional。
- Exit: provenance 和生成物一致；本地隔离矩阵及 cache gate 达到记录的发布标准；所有本任务修改已原子 commit/push；工作树 clean。
- Product Decision Delta: 汇总各 Phase 审计，不用实现结果反向扩展产品权威。
- W6 evidence（2026-09-02）：current overlay inventory 基于最终 index 重生为 883 路径，生成器 `--check`、全局 metadata validator 与 56 个同步脚本测试通过。8 项定向隔离运行精确复现用户已批准延期的 7 failed + 1 timeout，并已逐项记录 test name、签名、pristine 0.151 对照、生产路径和延期权威；受控 `-j 4` 全量为 3969 项中 3948 passed（1 flaky）、7 failed、14 timed out、9 skipped，额外 13 项为单列的当前宿主 zsh-fork/exec-wrapper 超时。PAT route 2/2、Guardian integration 2/2、websocket 42/42、spec plan 54/54、executor MCP 4/4、cache final-wire mock 2/2 通过。用户批准 1 CNY 总包后，R2 与持久证据 R3 的 Standard + map-request 双臂均业务成功、usage 完整、trace coverage 100%；累计 26 个 Provider 请求，实际估算费用 0.05895688 CNY。R3 绑定 `e39d5bd4…` 敏感面并已晋升 accepted baseline，发布级 live gate 通过。聚焦 Round 2 独立复审确认 B1/B2 均关闭、无 admissible blocker，Product Decision Delta 为 none。

## 10. Plan Delta Log

| ID | Before Phase | Previous Plan | Current Fact | Proposed Change | Impact | User Approval | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PDL1 | Phase B | 以旧的 292 条 overlay、115 个上游交集估算 W3–W5 replay 范围 | v0.0.6 最终 index 形成 883 条 overlay；0.151 有 306 个交集、64 个三方冲突和 2 个非冲突式 apply failure | 保持 W3/W4/W5 架构与顺序不变，改用新生成的 883 条 replay ledger 作为执行全集；每阶段只处理归属自身 batch 的路径 | 不增加产品功能或新架构，但机械 replay、人工复核和回归成本显著增加 | not-required（engineering-only，2026-09-01 用户确认） | accepted |
| PDL2 | Phase C | W3 只恢复 generic seam，Provider/TaskSpace 文件全部留到 W4/W5 | 0.151 的 app-server、TUI 和 state 编译边界直接引用既有 Provider credential、TaskSpace protocol 和 migration；完全延后会使 W3 无法形成可验证 substrate | W3 只提前恢复编译和既有入口连续性所需的最小接口，并处理 0051 migration 兼容；W4/W5 仍独立完成行为矩阵、schema 和产品语义验证 | 扩大 W3 的接口覆盖但不新增功能、不改变状态权威，也不把编译通过等同于 W4/W5 完成 | not-required（engineering-only） | accepted |
| PDL3 | Phase E | W6 笼统要求“刷新所有生成工件” | cutover overlay/replay 是切换前 index 的执行合同；在已切到 0.151 的 index 上重建会把上游新增路径误判为 Whale overlay，并产生 191 个错误分类 | 冻结 v0.0.5 历史工件和 v0.0.7 cutover 合同；新增只相对固定 0.151 substrate 计算的 current overlay inventory，validator 同时检查历史结构、cutover 内部一致性和 current overlay 可复现性 | 新增一个轻量 provenance 工件和生成入口，消除迁移历史与当前状态的双重含义；不触及运行时或产品行为 | not-required（engineering-only） | accepted |

每个 Phase 开始前必须在此记录物质变化，禁止静默改计划后继续执行。

## 11. Release And Rollback Boundary

- 每个 W 单元形成可独立理解、已做相称验证的提交并立即 push；机械 vendor、Provider/DeepSeek、TaskSpace/Extension、发布证据不得混为一个提交。
- 任何阶段失败优先 revert 当前阶段提交回到上一个 verified stop，不使用破坏性 reset。
- 0.151 只有在 W1–W6 全部 verified 后才可写入 v0.0.7 release identity；资格候选、局部编译或 mock 结果不得表述为生产集成完成。
- 对抗性审查属于高风险收口建议，但必须由用户另行批准后执行。
