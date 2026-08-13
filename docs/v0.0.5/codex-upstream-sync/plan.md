# Codex CLI 主线融合执行计划

- 文档状态：有效，Phase A、Phase B verified；下一步执行 Phase C rebase gate
- Plan Validity：`valid-with-qualifications`
- 计划性质：覆盖已完成里程碑与剩余工作的唯一执行计划
- 适用版本：WhaleCode v0.0.5
- 工作空间：仅 `/home/zhangxu/whalecode-codex`
- Product Authority：[./decisions.md](decisions.md)
- Applicable Decisions：D1、D2
- 当前生产 vendor：Codex CLI `rust-v0.147.0` / `be6e8eac029b183056b7e4402879f15d2c85f61b` + U3 最小 Whale substrate overlay（U4 已验证）
- 当前候选：U2 资格结论 `direction-supported-with-known-test-risks`；U4 免费 cache final-wire 门禁通过
- 官方发布依据：[OpenAI Codex Changelog：0.147.0（2026-08-07）](https://learn.chatgpt.com/docs/changelog)

## 1. 执行合同

- `decisions.md` 是本专题唯一产品决策权威源；active 决策只能由用户显式修改，Agent 禁止自我批准。
- 已验证的代码、测试、日志和上游事实可以修订本计划，但不能静默改写产品权威。
- 新出现的实质产品选择必须延期、局部 provisional，或交由用户确认；不得把工程实现倒推成产品决定。
- 每个实质阶段结束后，只审计该阶段的 Product Decision Delta，分类为 `covered`、`engineering-only`、`provisional` 或 `conflict`。
- 每个实质阶段开始前，必须根据已完成实现和验证证据重基剩余计划，并在该 Phase 的 `Pre-Phase Plan Rebase Gate` 中持久化结论。
- Phase gate 为 `pending` 或 `blocked-on-plan-approval` 时不得开始；material Plan Delta 必须记录并获得用户直接批准后才能应用和继续执行。
- 存在未解决的 material `provisional` 或 `conflict` 时，不得进入依赖它的下一阶段。
- 每个工作单元独立提交并 push；生成物必须与其权威源在同一单元生成和验证。
- 单元开始触及另一个产品域时立即停止并拆分，不通过新框架、双实现或临时业务分支维持进度。

## 2. 当前事实与治理结论

### 2.1 已完成工作与继承关系

| 已完成范围 | 完成度 | 已交付结果 | 对剩余计划的直接输入 | 状态解释 |
| --- | ---: | --- | --- | --- |
| 安全与通用 backport | 6/6 verified | 6 个独立上游修复已合入 | 当前 vendor 已包含这些补丁，后续不得重复回移 | 已完成 |
| 同步基线与测试门禁 | 12/15 verified；3 deferred | upstream 基线、overlay inventory、测试门禁；DeepSeek Responses 兼容、Flash 默认/Pro 隐藏已经落到当前 vendor | U5–U10 迁移的是这些既有行为在新 substrate 上的重放，不是重新设计或首次实现 | 已收口；TaskSpace TUI 已知夹具问题、Windows runner、Windows 终端 smoke 保持延期 |
| 0.146 资格与差异证据 | 9/9 completed；U1 verified | 候选身份、4,355 条 upstream delta、730 路径索引、两轮 qualification 日志；U1 修正 3 个 runner 问题并确认完整矩阵未通过 | 仅作为历史归因和 0.147 增量比较输入；不得充当 0.147 的身份、delta 或 qualification 证据 | U1 execution=`verified`；V1 validation=`direction-rejected` |
| 0.147 只读预检 | discovery completed | 官方 tag commit 已核验；相对 0.146 有 344 个提交、1,504 个变化路径；snapshot/Cargo.lock 仍保留开发版本 `0.0.0` | U2 checkpoint A 先复用 U1 runner 验证资格；仅 direction-supported 后由 checkpoint B 重算 target-dependent manifest、delta 和 replay 路由 | 只证明值得进入 U2，不证明候选合格或可 cutover |
| 0.147 正式资格 | U2 verified | fmt、offline CLI、sandboxed V8 code-mode-host 与 app-server 全过；core 3,288 passed / 5 path failures / 1 MCP timeout；TUI 3,376 passed / 33 release snapshots | 执行 Checkpoint B，刷新 0.147 target-dependent 工件后进入 U3 | `direction-supported-with-known-test-risks`；生产 vendor 未变化 |
| 0.147 target-dependent 工件 | Checkpoint B verified | overlay inventory 730 路径、upstream delta 4,666 路径、replay ledger 730 路径均固定到 0.147；app-server schema lineage 已跟随上游迁移到 Python 生成器 | 进入 U3 时按需查询路径证据，不把自动 disposition 当作产品决定 | 仅同步证据与生成脚本变化；生产 vendor tree 未变化 |
| 0.147 最小 Whale 兼容边界 | U3 verified | 6 个生产文件 + 2 个专用测试文件的临时 overlay 可构建 `whale 0.147.0`；home/auth/keyring 隔离通过；remote plugin/sharing 默认值锁回 false | Phase B 只重放声明的 substrate patch；DeepSeek、TaskSpace 仍留在各自单元 | 无生产 vendor 变化；0 模型请求 |
| 0.147 vendor substrate | U4 verified | vendor 替换只含 8 个 U3 修改路径；U4a 已独立迁移免费缓存合同，U4 cache index gate 通过 | Phase C 按独立工作单元重放 DeepSeek；TaskSpace 保留到 Phase D | 已纳入 U4 原子交付；0 模型请求 |
| 计划治理 | 1/1 verified | 唯一产品权威、唯一执行计划、历史工件降级和闭环工作单元 | 约束 U1–U17 的范围和停止条件 | 已完成 |

因此，U2 已按纠正后的官方构建合同完成，U3、U4 均已验证；`U5–U17` 尚未执行，不表示整个 Codex 主线追赶从零开始。历史工作、已完成单元与剩余工作目标、单位和验收标准不同，不做失真的简单平均百分比。

当前状态应读取为：

- 选择性上游修复：已完成；
- 同步基线、门禁和差异准备：已完成并带 3 项明确延期；
- 0.146 候选初次资格审查：已完成，结论 no-go；
- no-go 原因的最小增量复核：已完成（U1）；
- 0.147 候选正式资格：已完成（U2）；结论为 `direction-supported-with-known-test-risks`；
- 生产 vendor cutover：实现与门禁验证均已完成（U4）；DeepSeek/TaskSpace 重放尚未开始（U5–U17）。

### 2.2 治理后的权威关系

| 工件 | 治理后角色 | 可以证明 | 不可以决定 |
| --- | --- | --- | --- |
| `backport-ledger.json` | 已完成变更证据 | 哪些独立补丁已合入 | 后续架构迁移顺序 |
| `upstream-candidate.json` 与 qualification 日志 | 候选事实证据 | 当前记录 0.147 身份、官方 V8 构建合同、六项矩阵和已知测试风险；0.146 历史日志仍独立保留 | 未来稳定版自动合格，或已知失败自动等于 Whale 产品缺陷 |
| `overlay-inventory.json`、`upstream-delta-inventory.json` | 路径查询索引 | 路径、hash、双方是否变化 | 文件的产品语义或处理方式 |
| `overlay-replay-ledger.json` | 非权威路由提示 | 自动分类结果和待人工检查热点 | `adapt`、`drop`、owner 或 cutover 的最终决定 |
| 历史执行报告与 ledger | 已完成工作证据 | 当时做过什么、得到什么结果 | 当前或未来执行授权 |
| 本 `plan.md` | 唯一有效工程计划 | 当前单元、门禁和停止边界 | 修改 `decisions.md` 的产品行为 |

### 2.3 停止扩张的做法

- 不再扩展通用迁移 schema、路径状态机、owner registry 或新的同步框架。
- 不再要求 730 个路径先获得自动语义 disposition 才能迁移；只按当前工作单元查询。
- 不再用 brand/home、substrate、DeepSeek、TaskSpace、generated 五个大文件桶直接执行合并。
- 不为让同步测试全绿而修复既有业务问题；基线只证明本单元没有新增回归。
- 不把所有生成物拖到最终批次；schema、snapshot、锁文件与对应源变更同单元更新。
- 不创建第二份长期 vendor、运行时兼容框架、双状态权威或同步专用产品逻辑。

## 3. 目标、非目标与设计

### 3.1 目标

1. 证明固定的 0.147 候选可按其官方入口在隔离环境中构建和测试。
2. 以尽量接近上游原样的 vendor 为 substrate，只保留有产品依据的 Whale overlay。
3. 将 DeepSeek、缓存合同和 TaskSpace 分别作为独立闭环重放、验证和回滚。
4. 最终形成来源可追溯、行为可验证、下一次可重复的上游同步结果。

### 3.2 非目标

- 不重新设计 TaskSpace、Multi-Agent、Create/Debug Primitive 或模型分层。
- 不启用 Apps、Plugins、remote Code Mode、audio/image/realtime 等新增产品能力。
- 不因 0.147 新增 `--approve-for-me`、portable Agent Plugins、thread sections 或 MCP 2026-07-28 就默认启用相应产品行为。
- 不主动修复已登记的 TaskSpace TUI 夹具问题，也不把 Windows runner/终端 smoke 延期项表述为通过。
- 不访问、检查或管理其他分支和工作空间。
- 不进行真实 Whale Agent run；缓存门禁若要求真实回归，另按预算规则申请。

### 3.3 最小充分路径

```text
0.147 官方候选资格与目标工件刷新
  -> 临时树 identity/home 最小 overlay
  -> 生产 vendor substrate
  -> DeepSeek provider/catalog
  -> DeepSeek Responses wire
  -> cache/final-wire
  -> TaskSpace domain/data
  -> TaskSpace tool/session hooks
  -> protocol/app-server/TUI
  -> 发布闭环
```

默认修改现有 seam 或使用局部 adapter。只有证据证明上游没有稳定 seam，且重复宿主侵入会造成状态不一致时，才规划新抽象；若扩大当前单元，先停下重新审查。

### 3.4 Plan Delta 历史

| ID | Before Phase | Previous Plan | Current Fact | Proposed Change | Impact | User Approval | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PLD-001 | Phase A | 继续以 0.146 为候选；U2 验证 overlay；U3 替换 vendor | 0.147 已于 2026-08-07 发布，固定 commit 为 `be6e8eac029b183056b7e4402879f15d2c85f61b`；0.146 工件不能证明 0.147 | 当前候选改为 0.147；新增候选资格单元 U2；原后续单元顺延 | target-dependent 工件和资格证据必须重算；生产替换推迟到 U4 | `user-approved-plan-direct: “根据147正式更新计划”` | approved-applied |
| PLD-002 | Phase A | 无 phase rebase gate；U2 同时执行资格与全部工件重算；完整包测试未做环境能力预检 | 新版 `se-good-plan` 要求每个 material Phase 先重基；U1 已证明本机 sandbox/network 能力会制造同源噪声；0.147 新能力可能随整仓替换进入默认产品面 | 补齐五个 phase gate；U2 增加资格优先 checkpoint 和 sandbox preflight；U3 增加新能力默认暴露审计；校正 execution/validation 状态 | 不增加运行时架构；候选失败可更早停止；U4 前新增明确的权限/持久化/协议冲突门禁 | `user-approved-plan-direct: “根据审查结果先治理方案”` | approved-applied |
| PLD-003 | Phase B | U4 作为只含上游 substrate + U3 seam 的独立提交，随后才进入 DeepSeek/TaskSpace | cache gate 把旧 Whale final-wire/policy 的同批删除与 0.147 缓存敏感源码替换识别为硬冲突；当前无 DeepSeek/TaskSpace 的 U4 也不是有效真实回归主体 | 增加独立 U4a：在不改产品源码和 accepted baseline 的前提下治理 vendor-cutover 的 cache contract/提交边界；U4 重新通过免费门禁后再提交；真实 2-sample 回归推迟到 DeepSeek/TaskSpace 被测闭环恢复后 | 增加一个测试治理单元，但避免巨型 U4–U16 合并提交、无效付费运行或绕过门禁 | `user-approved-plan-direct: “批准”` | approved-applied |

## 4. 最低成本预投资验证

| ID | Critical Assumption | Decision Unlocked | Cheapest Credible Method | Enough Evidence / Not Proven | Budget / Isolation | Stop / Cleanup | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| V1 | 0.146 的四项失败是否主要来自错误命令、ambient proxy 或过严 `--locked` | 是否继续以 0.146 为候选 | 在临时官方树读取该 tag 自带 README/justfile/CI，清空非必要代理和 Whale 环境，执行官方入口 | 三个 runner 问题已证实并修正；完整 core/app-server/TUI 入口未全部完成 | 独立临时树和 target；0 模型请求 | 日志已保留；0.146 不再作为当前候选 | direction-rejected |
| V2 | 0.147 是否在复用 U1 runner 修正后具备进入 overlay 验证的资格 | 是否执行 U2 checkpoint B 和 U3 | 固定官方 commit；探测宿主能力；按候选自带 `setup-rusty-v8` 合同从 OpenAI 专用 release 下载并校验 archive/binding；再执行六项矩阵 | 官方资产、helper、CLI、app-server 已通过；core/TUI 剩余失败已归为硬编码 `/tmp`、MCP 时序与 release snapshot 风险 | 独立临时树和 target；0 模型请求；不写生产 vendor | 保留历次 evidence；风险进入后续回归归因，不修改候选规避 | direction-supported-with-known-test-risks |
| V3 | 0.147 substrate 能以很薄的 identity/home overlay 支撑 Whale CLI | 是否替换生产 vendor | 一次性临时候选树只应用品牌、二进制身份、`WHALE_HOME`、auth 隔离 patch | CLI build/version、home、direct keyring 与 encrypted secrets keyring 隔离均通过；不需要 DeepSeek/TaskSpace stub | 未提交第二份 vendor；0 模型请求 | 临时树已删除；证据落入 U3 report | validated |
| V4 | 0.147 新增用户可见能力不会在整仓替换时静默改变 Whale 默认权限、持久化或协议行为 | 是否执行 U4 | 在临时 0.147 tree 检查 `--approve-for-me`、portable Agent Plugins、thread sections、MCP 2026-07-28 的 CLI help、配置 schema、feature/default、protocol 和持久化入口 | approve flag 与 thread RPC 为显式动作；MCP 2026 默认 false；remote plugin/sharing 的上游默认 true 已通过现有 feature seam 锁回 false | 只读源码 + 本地无模型 smoke；不改生产候选 | 未新增禁用框架；临时树已删除；证据落入 U3 report | validated |

## 5. 可执行工作单元

### Phase A：候选方向验证

#### Pre-Phase Plan Rebase Gate

- Rebase scope：已完成 U1、0.147 官方身份和只读预检、当前同步脚本/metadata/schema、Phase A–E 剩余计划。
- Material plan delta：`material`
- Plan delta record：PLD-001、PLD-002
- User approval：`user-approved-plan-direct: “根据147正式更新计划”；“根据审查结果先治理方案”`
- Gate status：`ready`

进入条件：工作树 clean；生产 vendor 未变化；0.147 官方 commit `be6e8eac029b183056b7e4402879f15d2c85f61b` 可读取。适用决策：D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U1 | 增量复核候选 no-go 原因 | compatibility | 临时 0.146 tree；既有 candidate manifest/report | 已记录的四个失败与官方 build/test entrypoints | 已修 runner 的 offline lock 处理、代理隔离、local nextest 参数和显式 helper build；候选源码未改 | 三个初始失败归入 runner，TUI 归入不可变上游 fixture；完整矩阵仍失败 | 排除假 blocker，同时保留真实 no-go | 仅 runner、契约、日志、manifest 和报告；无生产影响 | 36 个 runner/metadata 单测；6 命令矩阵；vendor diff 为零；0 模型请求 | 已按失败停止条件收口 | verified |
| U2 | 正式验证 0.147 候选 | compatibility/evidence | `qualify_candidate.py`、`metadata_contract.py`、candidate schema/单测、0.147 candidate/evidence、临时 0.147 tree | candidate identity 与 qualification；Checkpoint A 期间 overlay target 保持 0.146 | 按候选官方 action 校验 Codex-built sandbox V8；补齐 sibling binary、HOME/TMP/umask 隔离并完成六项矩阵；direction-supported 后由 Checkpoint B 刷新查询工件 | 0.147 获得可复现的 `direction-supported-with-known-test-risks` 结论；生产 vendor 未变 | 修复验证方法而不修改候选源码或生产 vendor | runner 与 evidence 复杂度增加，但未进入运行时；无生产影响 | 40 个 runner 单测；官方 checksum；六命令矩阵；metadata validator；vendor index tree 不变；0 模型请求 | 已停止继续追求表面全绿；已知风险进入后续归因 | verified |
| U3 | 验证最小 Whale substrate 兼容边界 | compatibility | 一次性临时 0.147 candidate tree；CLI/config/protocol feature surfaces | brand、binary identity、`WHALE_HOME`、auth isolation；0.147 新能力默认状态 | 已执行 V3/V4；最小 patch 覆盖 CLI identity、home、两种 keyring service 和 remote plugin/sharing defaults | Whale 身份与数据隔离正确；新增 remote/sharing 不默认开启；无需 DeepSeek/TaskSpace stub | 在生产替换前同时验证最小 overlay 和用户可见兼容边界 | Complexity：6 个生产文件 + 2 个专用测试文件，无新框架；Reach/Cost：定向测试和 CLI smoke，0 模型请求 | fmt；home 6、login 25、secrets 9 及 3 个定向测试；CLI build/version/help/login smoke | 临时树已删除；生产 vendor 未变；证据见 U3 report | verified |

退出条件：已满足，U2、U3 均 verified；生产 vendor 仍保持当前版本。U1 保留为历史已完成单元。执行证据见 [0.147 qualification report](../../migration/codex-sync/2026-08-13-rust-0.147-candidate-qualification.md) 与 [U3 compatibility report](../../migration/codex-sync/2026-08-13-u3-minimal-substrate-compatibility.md)。

### Phase B：上游 substrate 落地

#### Pre-Phase Plan Rebase Gate

- Rebase scope：已复核 Phase A 的 0.147 qualification、target-dependent 工件、U3 最小 overlay/default 审计和 Phase B–E 剩余计划。
- Material plan delta：`material`（U4 执行后由 cache gate 新证据触发）
- Plan delta record：PLD-003
- User approval：`user-approved-plan-direct: “批准”`
- Gate status：`verified`

进入条件：Phase A verified，用户已看到 0.147 资格结论。适用决策：D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U4 | 替换 vendor substrate | dependency/source | `third_party/codex-cli/`、`UPSTREAM.md` | 已验证的 0.147 源码与 workspace metadata | 已用官方 Git archive 替换 vendor，只重放 U3 patch；保留官方锁文件 | vendor 对齐 0.147，Whale 数据仍隔离；DeepSeek/TaskSpace 未混入本单元 | 建立可维护 substrate，减少逐补丁漂移 | Complexity：大规模机械变化、无新业务抽象；Reach/Cost：全 workspace 构建和审查成本高 | SHA/tree/license、U3 tests、offline CLI build/smoke、42 个 metadata 单测、cache index gate 通过 | 独立 revert U4；产品 overlay 仍按 U5–U16 分单元恢复 | verified |
| U4a | 迁移 vendor cutover 缓存验证合同 | test/governance | cache gate contract、free final-wire fixtures、U4 提交边界 | 0.147 已移除的 Whale 专用 cache policy/final-wire 合同 | 已在不修改产品源码、accepted baseline 或真实回归结果的前提下迁移免费合同 | U4 的缓存敏感变化可被免费证据发现并与后续真实回归边界分离 | 保留强制门禁，同时避免无被测链路的付费运行 | Complexity：限于测试/门禁；Reach/Cost：增加免费测试维护成本，0 模型请求 | 53 个 gate/contract 单测；free final-wire 全矩阵；index gate 通过；accepted baseline 未变 | 独立提交 `17424eac8` 可回滚；不得据此晋升 live baseline | verified |

退出条件：已满足。U4 常规验证与 cache index gate 均通过；Phase B 未重放 DeepSeek/TaskSpace，也未改变 accepted live baseline。证据见 [U4 vendor cutover report](../../migration/codex-sync/2026-08-13-u4-vendor-substrate-cutover.md)。

### Phase C：DeepSeek 闭环

#### Pre-Phase Plan Rebase Gate

- Rebase scope：U4 实际 vendor substrate、编译缺口、上游 provider/catalog/Responses/cache seam + Phase C–E 剩余计划。
- Material plan delta：`pending`
- Plan delta record：`pending`
- User approval：`pending-if-material`
- Gate status：`pending`

进入条件：U4 verified。适用决策：D1、D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U5 | 恢复 provider 身份与鉴权 | provider | `model-provider-info` 及配置调用点 | DeepSeek provider identity、base URL、API key auth | 按新上游 provider 接口重放现有身份和鉴权逻辑，不引入模型目录与请求 payload | CLI 能识别并配置 DeepSeek provider | 先恢复最小接入点，隔离目录和 wire 问题 | Complexity：一个局部 provider 分支；Reach/Cost：配置解析和 auth tests | provider/config/auth 定向测试；0 真实请求 | 独立 revert；模型能力留给 U6 | not-started |
| U6 | 恢复模型目录与可见性 | catalog/defaults | `models-manager` 及 model selector | Flash/Pro catalog entries、default/visibility | 迁移模型元数据，保持 Flash 默认可见、Pro 隐藏；同单元更新目录生成物和快照 | 模型选择行为符合 D1 | 单独审计用户可见默认值 | Complexity：目录数据和现有 selector 条件；Reach/Cost：生成物和 TUI 快照 | model manager tests；默认/可见性快照；D1 检查 | 独立 revert；不得顺手实现 wire | not-started |
| U7 | 恢复 Responses 请求与流式事件 | provider-wire | `codex-api`、`core` 的 Responses/SSE seam | request mapping、reasoning stream、tool-call assembly | 按上游类型适配现有请求和事件组装，不触及 TaskSpace、用量、压缩或缓存 policy | reasoning 与 streamed tool calls 保持现行为 | 恢复主请求链并把后续状态行为隔离 | Complexity：Responses/SSE 局部分支；Reach/Cost：API/core fixtures | 无网络 request contracts、SSE reasoning/tool-call fixtures | 出现 TaskSpace 数据需求即停止并留给 U13/U14 | not-started |
| U8 | 恢复用量与请求预算 | accounting | `core/src/client.rs` 现有 accounting seam | provider usage、hard request limit、terminal reconciliation | 将现有计数与限额接到上游响应/事件类型 | 请求用量和终止对账保持一致 | 防止主线融合丢失成本与请求保护 | Complexity：局部计数状态；Reach/Cost：client/session accounting tests | usage fixtures、hard-limit、terminal reconciliation tests | 独立 revert；不修改 compaction threshold | not-started |
| U9 | 恢复 DeepSeek compaction | context-lifecycle | `core/src/compact*.rs` 及现有调用点 | Flash compact request、1M/755K threshold、保留状态 | 按新上游 context API 迁移现有 compaction 合同，不引入 TaskSpace projection 改动 | 长上下文收缩保持现有阈值和状态保留 | 将上下文生命周期风险从 wire/cache 中隔离 | Complexity：复用现 compaction 分支；Reach/Cost：context/session tests | compact request、threshold、retention、zero-short-job tests | 若必须修改 TaskSpace retention，停下留给 U14 | not-started |
| U10 | 恢复缓存与 final-wire 证据 | cache/observability | `provider_wire_*`、cache contract/gate | Standard 最终 payload 与敏感面 | 迁移 free final-wire/trace，运行 index gate；不建新缓存框架 | 可在零模型请求下检测 payload 漂移 | 付费回归前提供确定性保护 | Complexity：复用现合同、原则上无新 runtime state；Reach/Cost：cache-sensitive，可能预算阻塞 | contract tests；cache index gate；真实回归须另获批 | gate 阻断则回退/停在 U10，不用 `--no-verify` | not-started |

退出条件：U5–U10 verified；D1 语义 covered；没有 TaskSpace 产品语义变化。

### Phase D：TaskSpace 闭环

#### Pre-Phase Plan Rebase Gate

- Rebase scope：U4 substrate 与 U5–U10 DeepSeek 实现、TaskSpace 实际编译/测试状态、上游 domain/data/tool/session/client seam + Phase D–E 剩余计划。
- Material plan delta：`pending`
- Plan delta record：`pending`
- User approval：`pending-if-material`
- Gate status：`pending`

进入条件：Phase C verified。适用决策：D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U11 | 恢复 TaskSpace 领域模型 | domain | `core/src/action_map/**`、独立 protocol/tools 类型 | canonical map、DAG invariant、event types、tool schema | 重放不依赖存储/session/TUI 的领域模块，按新 workspace 类型做编译适配 | TaskSpace 业务模型可独立构建和验证 | 先固定业务含义，避免宿主接口反向塑形 | Complexity：保留现模块、不加新抽象；Reach/Cost：domain/protocol unit tests | invariant、serialization、tool schema tests | 若需改变状态语义，标 conflict 并请示 | not-started |
| U12 | 恢复持久化与 replay | data | state runtime、SQL migration、TaskSpace store | event store、CAS、migration、replay | 将 U11 类型接回现有 store；migration/schema 同单元生成 | canonical state 可持久化、迁移和重放 | 单独验证任务状态唯一权威 | Complexity：不建第二 store；Reach/Cost：SQL/schema/migration tests | CAS/store、migration upgrade、replay determinism tests | 无法保持单一权威则标 conflict 并请示 | not-started |
| U13 | 接回 tool identity/sequence | tool-runtime | core/tools registry、handler、sequence seam | control tool、call identity、preflight/terminal carrier | 在上游 seam 注册现有 handler，适配 call identity/response index | TaskSpace 控制调用恢复 | 从 session 生命周期中隔离工具冲突 | Complexity：窄 adapter/hook；Reach/Cost：registry、parallel calls、错误传播 | schema/handler、sequence、parallel、terminal tests | 不改 session/store 语义；所需内容留给 U14 | not-started |
| U14 | 接回 session/resume/fork | lifecycle | core/session、rollout reconstruction seam | projection、resume/fork/replay integration | 通过上游 session seam 投影 U12 状态，适配 resume/fork/compaction | 跨轮次行为可恢复和回放 | 完成核心生命周期并保持单一权威 | Complexity：session adapter；Reach/Cost：history、compaction、agent identity | resume/fork/replay/compaction/terminal tests；权威断言 | 若需并列状态权威，标 conflict 并请示 | not-started |
| U15 | 恢复 app-server 协议 | API | protocol、app-server TaskSpace adapter | RPC、JSON/TS schema、compatibility surface | 将 U11–U14 暴露到现有 RPC；schema/TS 与源同单元生成 | 外部客户端可通过版本化协议访问 TaskSpace | 单独验证协议兼容，不混 TUI 路由 | Complexity：现有 RPC adapter；Reach/Cost：schema generation 和 app-server tests | protocol/app-server tests；generation clean；兼容 fixtures | 独立 revert；不得引入 TUI fallback | not-started |
| U16 | 恢复 TUI 路由与 viewer | client | TUI TaskSpace route/viewer | route state、Action Map viewer、快照 | 按新 TUI seam 接入已验证 RPC/core 行为；快照同单元更新 | TaskSpace 在终端中保持现有可见交互 | 完成用户可见闭环并隔离已知夹具问题 | Complexity：现有 TUI adapter；Reach/Cost：route tests 和快照 | route/viewer tests；snapshot review；已知 TaskSpace 夹具失败按基线记录 | 不得为消除既有失败而改变业务路由规则 | not-started |

退出条件：U11–U16 verified；状态权威无冲突；TaskSpace TUI 已知夹具失败未新增回归且继续明确延期。

### Phase E：发布收口

#### Pre-Phase Plan Rebase Gate

- Rebase scope：U4–U16 实际实现、生成物、Linux/Windows 延期、缓存门禁和剩余发布风险 + Phase E 计划。
- Material plan delta：`pending`
- Plan delta record：`pending`
- User approval：`pending-if-material`
- Gate status：`pending`

进入条件：Phase D verified。适用决策：D1、D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U17 | 完成发布闭环 | release | workspace metadata、`UPSTREAM.md`、migration report、CI/release config | 来源、剩余生成物、支持矩阵 | 刷新机械生成物和来源记录，跑 Linux 全量无模型回归，明确 TaskSpace TUI/Windows deferred | 形成可追溯新 vendor 和真实通过/延期清单 | 防止局部通过误报为整体完成 | Complexity：无新生产抽象；Reach/Cost：全 workspace build/CI | fmt/check/test、CLI smoke、schema/lock clean、cache gate、provenance、Git clean | 跨产品失败回到归属 U 单元，不在收口阶段扩张 | not-started |

退出条件：U17 verified；所有修改已提交并 push；工作树 clean；延期项未被表述为通过。

## 6. Product Decision Delta

每个阶段只追加一行，不回写 `decisions.md`。

| Phase | Decision Surface | Implemented / Observed Semantics | Authority Coverage | Classification | Required Action |
| --- | --- | --- | --- | --- | --- |
| 已完成：安全 backport | 无产品语义 | 独立安全/通用修复 | none | engineering-only | 已收口 |
| 已完成：基线与门禁 | 模型默认值与 Responses | Flash 默认、Pro 隐藏；Responses 按能力处理 | D1、D2 | covered | 保留为现行为证据，不作未来授权 |
| 已完成：资格与差异证据 | 无生产语义 | 仅候选、差异和 replay 证据；vendor 未变 | none | engineering-only | 自动语义分类已降级 |
| 已完成：U1 | 无产品语义 | 修正 qualification runner 并确认 0.146 validation direction-rejected；vendor 未变 | D2 | engineering-only | 已收口为 Phase A 历史输入 |
| Phase A | 已完成 | U2 direction-supported-with-known-test-risks；Checkpoint B 已刷新 0.147 查询工件；U3 最小 identity/home/auth/default seam 验证通过；vendor 未变 | D2 | covered + engineering-only | 进入 Phase B rebase gate |
| Phase B | 已完成 | vendor 对齐 0.147 且只保留 U3 seam；U4a 独立迁移免费缓存合同，cache index gate 通过；live baseline 仍保持失败状态 | D2 | covered + engineering-only | 进入 Phase C rebase gate |
| Phase C | 待执行 | 待记录 | D1、D2 | 待分类 | U5–U10 后审计 |
| Phase D | 待执行 | 待记录 | D2 | 待分类 | U11–U16 后审计 |
| Phase E | 待执行 | 待记录 | D1、D2 | 待分类 | U17 后审计 |

## 7. Pending Product Decisions

当前没有新的产品决策；U2 仅修正工程验证方法。以下情况出现时必须停下请示：

- upstream AgentGraph/WorldState 与 TaskSpace Event Store 无法保持单一任务状态权威；
- 后续拟偏离候选自带的官方 release/CI 构建合同，以本地源码构建或替换依赖规避真实发布缺口；
- DeepSeek 官方 Pro Responses 支持状态变化，可能触发 D1 恢复条件；
- 新上游能力会改变默认权限、持久化、模型可见性或用户控制方式。

## 8. 执行与提交边界

执行顺序当前到达 `... -> U3（已完成） -> Phase B rebase gate（已完成） -> U4a（已完成） -> U4（已验证）`。下一步进入 Phase C rebase gate；在 DeepSeek/TaskSpace 被测路径恢复前不消耗真实回归预算，也不晋升 live baseline。

- 每个 U 单元至少一个独立、可理解、已基本验证的 commit，并立即 push。
- 单元内出现两个可独立回滚的行为主题时继续拆 commit；不得把 vendor 机械替换与产品 overlay 混为一个提交。
- 本计划不授权新分支、真实模型请求、跨工作空间操作或超过仓库规模门禁的实现。
- 每阶段完成后先更新状态、证据链接和 Product Decision Delta，再进入下一阶段。
- 每个 Phase 开始前先完成其 `Pre-Phase Plan Rebase Gate`；若 delta=`none`，记录 `User approval: not-required` 并置 `ready`；若为 `material`，先记录 delta、置 `blocked-on-plan-approval`，获得用户直接批准后再应用修订并置 `ready`。

## 9. 计划验收

- [x] 只有一个产品决策权威源，且只含用户直接确认的 active 决策。
- [x] 历史批次与自动账本已从执行权威降级为证据。
- [x] 候选资格在大投入前先做最低成本复核。
- [x] 每个工作单元只有一个主要目标、变更轴和回滚边界。
- [x] DeepSeek provider/catalog、Responses wire、cache 分开验证。
- [x] TaskSpace domain/data、tool、session、client surface 分开验证。
- [x] 生成物与对应源变化同单元处理。
- [x] TaskSpace TUI 已知夹具问题和 Windows 延期不阻塞 Linux 主线融合，也不被误报为通过。
- [x] 不新增运行时框架、双 vendor、双状态权威或同步专用业务逻辑。
- [x] 每个 material Phase 持久化 Pre-Phase Plan Rebase Gate；Phase A 已获直接批准并 ready，后续 Phase 保持 pending。
- [x] 0.146→0.147 及本轮治理的 material Plan Delta 已保留用户批准记录。
- [x] U2 按资格优先 checkpoint 执行，候选不支持时不重算 target-dependent 大工件。
