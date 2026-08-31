# WhaleCode v0.0.7 Codex CLI 0.151 主线追赶计划

- Status: blocked-on-plan-approval（Phase B）
- Plan Validity: valid-with-qualifications
- Created: 2026-08-31
- Product Authority: [`prd/2026-08-23-v0.0.6-multi-provider.md#confirmed-product-decisions`](../../../../prd/2026-08-23-v0.0.6-multi-provider.md#confirmed-product-decisions)
- Applicable Decisions: PD3、PD4、PD5、PD6、PD8、PD9、PD10、PD11、PD12、PD13、PD15、PD17、PD18、PD19
- Current vendor: `rust-v0.149.0` / `758ef40f50c1a458425c7cfbf1eb12cbc07af0b0`
- Target vendor: `rust-v0.151.0` / `78c290807ce710180111df227df3b7a4fe845452`
- Execution authorization: user-approved-execution-direct: 2026-09-01 “开始推进”

## 1. Execution Contract

- 上述 Product Authority 是本主题物质产品决策的唯一用户权威；修改、重释或替换 active 决策必须取得用户明确批准，Agent 不得自批。
- 已验证的代码、测试、上游源码和运行证据可以修订本计划，但不得静默改写 Product Authority。
- 新出现的用户可见产品选择必须延期、标记为 provisional，或交由用户确认；物质 `provisional` / `conflict` 会阻断依赖工作。
- 每个物质 Phase 结束后只审计该阶段的 Product Decision Delta；每个物质 Phase 开始前必须用已完成实现和证据 rebase 全部剩余计划。
- Pre-Phase Plan Rebase Gate 为 `pending` 或 `blocked-on-plan-approval` 时不得开始该 Phase。
- 设计方向、范围、模块/API/状态边界、工作单元、顺序、验证、回滚、收益、成本或风险发生物质变化时，必须记录 Plan Delta 并获得用户明确批准后才能继续。
- 不创建新分支；不触碰其他工作空间；不自动触发 GitHub Actions。真实模型运行仍受账本和预算门禁约束，本计划本身不授权任何付费请求。

## 2. Current And Expected Behavior

### 2.1 当前事实

- 当前分支已 rebase 并与 `origin/main` 对齐在 `a3ac0770df153dea2a18ff7e3cc5df245c76f45a`；v0.0.6 已发布。
- 当前 Codex vendor 固定在官方 0.149，已叠加 Whale identity、workspace isolation、多 Provider、DeepSeek Responses/cache 与 TaskSpace relational state overlay。
- 官方最新稳定目标 0.151 的 peeled commit 为 `78c290807ce710180111df227df3b7a4fe845452`，tree 为 `68d61fd9886a749a78487d8ce950e3cb9309a3d7`；0.152 仍是 alpha，不在本主题范围。
- 0.149→0.151 官方差分为 1,660 个路径；v0.0.6 最终 Git index 相对 0.149 有 883 个 Whale overlay 路径，其中 306 个与上游差分相交。只读三方试应用发现 64 个明确冲突文件，另有 1 个 shell handler 无法直接套用。
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
| V2 | 当前 overlay 能按既有 seam 重放而不新建第二套架构 | W3–W5 的分域边界 | 生成 0.149→0.151 delta、0.151 overlay inventory/replay，复核全部交集和冲突归属 | 足够：306 个交集均有唯一 batch、验证和 safe stop；64 个三方冲突与 1 个硬套用失败已单列；不证明运行时正确 | 纯 Git/脚本、0 请求 | 分类无法保持单一状态/route 权威时阻断对应 Phase | supported-with-expanded-scope |
| V3 | 0.151 的安全/效率收益没有被后续提交回退 | 是否需要额外 cherry-pick | 在固定 0.151 tree 中核对关键 commit 可达性和最终测试；不先 cherry-pick 到 production | 足够：目标 tree 包含 `035295b46e` sandbox/MCP/approval、`0182ff3480` model-aware ToolRouter、`bf3eb2ec91` PTY shutdown、`6677fd827d` / `528fd7ace5` retained-image budgeting、`5bf0ba3dd6` MCP result hook；不证明 Whale 合入后通过 | 只读源码和本地测试 | 缺失则从计划移除该收益，不从 alpha 补丁偷渡 | supported |

## 8. Work Units

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W1 | 固定并资格验证 0.151 候选 | qualification | `scripts/codex-upstream/qualify_candidate.py`、candidate metadata/evidence | tag/commit/tree、V8/toolchain、pristine test manifest | 将候选合同从 0.149 更新到固定 0.151，并在临时目录运行既有隔离矩阵 | 获得不触碰 production vendor 的 go/no-go 证据 | 在大规模 cutover 前排除无效目标 | Complexity：只改既有常量、fixture 和证据；Reach/cost：本地构建与长测试，0 模型请求 | tag/commit/tree/license、sync script tests、fmt、CLI、code-mode-host、core/app-server/TUI candidate logs | 任一方向性阻断即保留 0.149；W1 可独立 revert | verified |
| W2 | 生成可执行 replay 合同 | provenance/analysis | `scripts/codex-upstream/`、`docs/releases/v0.0.7/codex-upstream-sync/` | upstream delta、overlay inventory、replay batches、conflict ledger | 以 0.151 为目标生成路径级工件，并把冲突归入 generic、provider/DeepSeek、TaskSpace/extension、generated 四类 | 后续每个冲突只有一个 owner 和验证边界 | 降低现场误合并和跨域重构风险 | Complexity：metadata/文档，无运行时分支；Reach/cost：Git 分析、人工审阅，0 请求 | generator reproducibility、schema validator、path counts、三方 apply check | 分类不闭合则不进入 cutover；production vendor 保持 0.149 | verified |
| W3 | 导入 0.151 substrate 与通用 Whale seam | vendor/generic | `third_party/codex-cli/` 通用 build、identity、home、workspace、sandbox、exec/PTY 路径 | 官方 tree、Whale CLI/workspace seam、安全和效率修复 | 以官方 tree 机械替换 substrate，按 W2 replay 最小 identity/workspace seam；不在本单元设计 Provider 或 TaskSpace 新语义 | 通用底座升至 0.151，并获得权限、安全、shutdown 与执行效率修复 | 避免逐提交 cherry-pick 形成长期漂移 | Complexity：大规模机械 vendor 变化，上游文件豁免普通 500 行限制；Reach/cost：build/sandbox/exec/TUI 基础路径，0 请求 | conflict-free index、fmt/check、CLI identity、workspace doctor、权限/sandbox/PTY/unified-exec 定向测试 | 单独 commit；失败可 revert W3 回到完整 0.149，不在半冲突状态提交 | not-started |
| W4 | 恢复 Multi-Provider 与 DeepSeek 合同 | provider/cache/context | login、model-provider-info、models-manager、core client/session/ToolRouter/compaction、TUI provider/model | route-bound auth、model catalog、prepared transition、history projection、DeepSeek Responses/final-wire | 在 0.151 seam 上重放 v0.0.6 provider overlay，吸收 model-aware ToolRouter/reasoning 修复；retained-image budgeting 只对支持的 route 生效，DeepSeek 保持本地压缩 | 三访问路由和 DeepSeek 三模型行为不退化，同时获得上游模型切换修复 | 直接保护当前主要产品链和缓存正确性 | Complexity：只改既有 route/capability seam，不新增 mega-registry；Reach/cost：凭据、模型、请求、压缩、cache，可能触发真实 revalidation 申请 | login/model/provider/core/TUI 定向测试、DeepSeek SSE、provider transition、history projection、Standard final-wire、`check_cache_regression_gate.py --source index` | 免费 gate 要求真实 revalidation 时立即停止并申请预算；W4 commit 可独立 revert 到 W3 substrate 状态 | not-started |
| W5 | 恢复 TaskSpace、Extension 与 app-server/TUI 组合链 | taskspace/extension/protocol | state、tools、core extension lifecycle、app-server protocol/server、TUI routing/viewer、generated schema | relational store、fork/resume/reload、MCP result lifecycle、TaskSpace RPC/events | 在单一 relational state authority 上适配 0.151 session/extension 协议，生成 schema；PPD1 未确认前不激活冲突的 task UI | TaskSpace 与 extension 组合链在新 substrate 上可恢复、可重放 | 保留差异化能力并吸收 MCP result hook | Complexity：适配既有 extension seam，不建第二 store；Reach/cost：state/core/app-server/TUI/schema，0 请求 | state/tools/core TaskSpace、fork→reload→request、extension lifecycle/MCP result、app-server schema、TUI routing/viewer | 发现双状态权威或 PPD1 依赖即阻断相关入口；W5 独立 commit/revert | not-started |
| W6 | 发布资格与证据收口 | release/metadata | provenance、replay、UPSTREAM、release report、cache evidence | 0.151 vendor identity、完整受控矩阵、延期边界 | 刷新所有生成工件，运行当前 vendor 隔离回归与静态门禁，记录非绿项真实签名并提交推送 | v0.0.7 获得可审计的 0.151 substrate 候选 | 防止“版本已改但证据未跟上” | Complexity：文档/metadata/generated，无新产品逻辑；Reach/cost：长时间本地测试，真实模型费用默认 0 | metadata/delta/inventory/replay check、schema、fmt、isolated affected/full matrix、cache gate、clean tree、remote sync | 未达到退出条件不宣称完成；真实验证仍需单独预算和账本 | not-started |

## 9. Phases

### Phase A：候选资格与 replay 合同（W1–W2）

#### Pre-Phase Plan Rebase Gate

- Rebase scope: `main@a3ac0770d`、0.149 vendor、v0.0.6 已发布实现、固定 0.151 tag/commit/tree、当前 overlay/diff 证据
- Material plan delta: none
- Plan delta record: not-required
- User approval: user-approved-execution-direct: 2026-09-01 “开始推进”
- Gate status: ready

- Entry: 工作区 ready、工作树 clean、固定 tag 可达。
- Exit: V1–V3 有证据结论；W1–W2 verified；production vendor 仍为 0.149。
- Product Decision Delta: 预期 `engineering-only`；不改变用户可见行为。
- W1 evidence（2026-09-01）：固定 0.151 commit/tree、license、Rust 1.95 与官方 `rusty_v8 150.4.0` 合同通过；fmt、CLI offline check、code-mode-host build 3/3 通过。pristine core 为 3808/3815、7 failed、9 skipped；其中 6 个失败与 0.149 同签名，新增 1 个 cyber access timeout。app-server 为 1384/1389、1 failed、4 zsh-fork timed out、2 skipped；唯一普通失败与 0.149 的 external-agent-config 签名相同。TUI 为 3955/3982、27 failed、6 skipped，27 个失败与 0.149 的宿主/快照集合一致。结论为 `direction-supported-with-known-test-risks`；production vendor index tree 未改变，模型请求 0。
- W2 evidence（2026-09-01）：生成并复验 883 路径 overlay、1,660 路径 upstream delta、883 条 replay decision 和覆盖 306 个交集的 conflict ledger；其中 241 个可三方 clean apply、64 个三方冲突、1 个 shell handler 硬套用失败。五个 batch 分别拥有 146/274/73/162/228 条 replay decision，所有路径均有唯一 owner、disposition 与 verification。全局 sync metadata validator 和 56 个脚本测试通过；production vendor 仍为 0.149，模型请求 0。

### Phase B：0.151 substrate cutover（W3）

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase A 证据、最终 replay batches、当前 production vendor
- Material plan delta: PDL1（replay 范围由旧清单 292/115 扩为真实 883/306）
- Plan delta record: PDL1
- User approval: pending
- Gate status: blocked-on-plan-approval

- Entry: Phase A verified，方向为 supported，所有通用冲突已归属。
- Exit: vendor 指向 0.151；通用 build/identity/workspace/security/exec 定向验证通过；Provider/TaskSpace 未被误宣称完成。
- Product Decision Delta: 审计是否出现默认权限、可见能力或自动化变化。

### Phase C：Multi-Provider 与 DeepSeek 重放（W4）

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase B 实际代码、0.151 provider/session/ToolRouter/compaction seam、剩余 W4–W6
- Material plan delta: pending
- Plan delta record: pending
- User approval: pending-if-material
- Gate status: pending

- Entry: Phase B verified；现有 PD 可在 0.151 seam 上保持。
- Exit: provider/DeepSeek 定向矩阵与免费 cache gate 通过，或按门禁明确停在预算审批前。
- Product Decision Delta: 对照 Applicable Decisions 逐项审计 route、凭据、模型、history、compaction 和持久化。

### Phase D：TaskSpace、Extension 与 TUI 组合链（W5）

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase C 实际代码、0.151 extension/app-server/TUI seam、PPD1 状态、剩余 W5–W6
- Material plan delta: pending
- Plan delta record: pending
- User approval: pending-if-material；PPD1 相关激活需要用户决策
- Gate status: pending

- Entry: Phase C verified；TaskSpace 仍可保持单一 relational state authority。
- Exit: state/tools/core/app-server/TUI 组合链闭合；schema 可复现；PPD1 未决入口保持延期。
- Product Decision Delta: 审计任务概念、命令入口、extension tool result 顺序和持久化语义。

### Phase E：发布资格与收口（W6）

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase A–D 全部实现和证据、当前测试基线、剩余发布边界
- Material plan delta: pending
- Plan delta record: pending
- User approval: pending-if-material
- Gate status: pending

- Entry: W1–W5 verified；无未解决的物质 conflict/provisional。
- Exit: provenance 和生成物一致；本地隔离矩阵及 cache gate 达到记录的发布标准；所有本任务修改已原子 commit/push；工作树 clean。
- Product Decision Delta: 汇总各 Phase 审计，不用实现结果反向扩展产品权威。

## 10. Plan Delta Log

| ID | Before Phase | Previous Plan | Current Fact | Proposed Change | Impact | User Approval | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PDL1 | Phase B | 以旧的 292 条 overlay、115 个上游交集估算 W3–W5 replay 范围 | v0.0.6 最终 index 形成 883 条 overlay；0.151 有 306 个交集、64 个三方冲突和 1 个硬套用失败 | 保持 W3/W4/W5 架构与顺序不变，改用新生成的 883 条 replay ledger 作为执行全集；每阶段只处理归属自身 batch 的路径 | 不增加产品功能或新架构，但机械 replay、人工复核和回归成本显著增加；Phase B 前需重新确认继续投入 | pending | approval-required |

每个 Phase 开始前必须在此记录物质变化，禁止静默改计划后继续执行。

## 11. Release And Rollback Boundary

- 每个 W 单元形成可独立理解、已做相称验证的提交并立即 push；机械 vendor、Provider/DeepSeek、TaskSpace/Extension、发布证据不得混为一个提交。
- 任何阶段失败优先 revert 当前阶段提交回到上一个 verified stop，不使用破坏性 reset。
- 0.151 只有在 W1–W6 全部 verified 后才可写入 v0.0.7 release identity；资格候选、局部编译或 mock 结果不得表述为生产集成完成。
- 对抗性审查属于高风险收口建议，但必须由用户另行批准后执行。
