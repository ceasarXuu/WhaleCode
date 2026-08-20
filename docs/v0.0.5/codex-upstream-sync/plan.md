# Codex CLI 主线融合执行计划

- 文档状态：有效，Phase A–E 与 U18 verified；Phase F（U19）in-progress
- Plan Validity：`valid-with-qualifications`
- 计划性质：覆盖已完成里程碑与剩余工作的唯一执行计划
- 适用版本：WhaleCode v0.0.5
- 工作空间：仅 `/home/zhangxu/whalecode-codex`
- Product Authority：[./decisions.md](decisions.md)
- Applicable Decisions：D1、D2
- 当前生产 vendor：Codex CLI `rust-v0.147.0` / `be6e8eac029b183056b7e4402879f15d2c85f61b` + 已验证 Whale identity、DeepSeek Responses/cache 与 TaskSpace overlay（U18 已收口）
- 当前追赶目标：Codex CLI `rust-v0.149.0` / `758ef40f50c1a458425c7cfbf1eb12cbc07af0b0`
- 官方发布依据：[OpenAI Codex 0.149.0 Release（2026-08-20）](https://github.com/openai/codex/releases/tag/rust-v0.149.0)

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
| 同步基线与测试门禁 | 12/15 verified；3 deferred | upstream 基线、overlay inventory、测试门禁；DeepSeek Responses 兼容、Flash 默认/Pro 隐藏曾在 U4 前的 vendor 验证并交付 | U5–U10 在 0.147 substrate 上重放这些既有语义，并按 PLD-004 淘汰旧 Chat Completions 转换、恢复正式版 Pro | 已收口；TaskSpace TUI 已知夹具问题、Windows runner、Windows 终端 smoke 保持延期 |
| 0.146 资格与差异证据 | 9/9 completed；U1 verified | 候选身份、4,355 条 upstream delta、730 路径索引、两轮 qualification 日志；U1 修正 3 个 runner 问题并确认完整矩阵未通过 | 仅作为历史归因和 0.147 增量比较输入；不得充当 0.147 的身份、delta 或 qualification 证据 | U1 execution=`verified`；V1 validation=`direction-rejected` |
| 0.147 只读预检 | discovery completed | 官方 tag commit 已核验；相对 0.146 有 344 个提交、1,504 个变化路径；snapshot/Cargo.lock 仍保留开发版本 `0.0.0` | U2 checkpoint A 先复用 U1 runner 验证资格；仅 direction-supported 后由 checkpoint B 重算 target-dependent manifest、delta 和 replay 路由 | 只证明值得进入 U2，不证明候选合格或可 cutover |
| 0.147 正式资格 | U2 verified | fmt、offline CLI、sandboxed V8 code-mode-host 与 app-server 全过；core 3,288 passed / 5 path failures / 1 MCP timeout；TUI 3,376 passed / 33 release snapshots | 执行 Checkpoint B，刷新 0.147 target-dependent 工件后进入 U3 | `direction-supported-with-known-test-risks`；生产 vendor 未变化 |
| 0.147 target-dependent 工件 | Checkpoint B verified | overlay inventory 730 路径、upstream delta 4,666 路径、replay ledger 730 路径均固定到 0.147；app-server schema lineage 已跟随上游迁移到 Python 生成器 | 进入 U3 时按需查询路径证据，不把自动 disposition 当作产品决定 | 仅同步证据与生成脚本变化；生产 vendor tree 未变化 |
| 0.147 最小 Whale 兼容边界 | U3 verified | 6 个生产文件 + 2 个专用测试文件的临时 overlay 可构建 `whale 0.147.0`；home/auth/keyring 隔离通过；remote plugin/sharing 默认值锁回 false | Phase B 只重放声明的 substrate patch；DeepSeek、TaskSpace 仍留在各自单元 | 无生产 vendor 变化；0 模型请求 |
| 0.147 vendor substrate | U4 verified | vendor 替换只含 8 个 U3 修改路径；U4a 已独立迁移免费缓存合同，U4 cache index gate 通过 | Phase C 按独立工作单元重放 DeepSeek；TaskSpace 保留到 Phase D | 已纳入 U4 原子交付；0 模型请求 |
| 计划治理 | 1/1 verified | 唯一产品权威、唯一执行计划、历史工件降级和闭环工作单元 | 约束 U1–U17 的范围和停止条件 | 已完成 |

因此，U1–U17 均已执行并按各自证据收口。历史工作、已完成单元与延期验证的目标、单位和验收标准不同，不做失真的简单平均百分比。

当前状态应读取为：

- 选择性上游修复：已完成；
- 同步基线、门禁和差异准备：已完成并带 3 项明确延期；
- 0.146 候选初次资格审查：已完成，结论 no-go；
- no-go 原因的最小增量复核：已完成（U1）；
- 0.147 候选正式资格：已完成（U2）；结论为 `direction-supported-with-known-test-risks`；
- 生产 vendor cutover：实现与门禁验证均已完成（U4）；DeepSeek provider、原生 Responses、长上下文、final-wire/cache 与模型目录闭环均已恢复/验证（U5–U10）；TaskSpace extension、state、RPC、TUI/viewer 与 final-wire 已恢复/验证（U11–U16）；发布闭环已完成（U17）。

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

Phase C 的 PLD-004 只采用 DeepSeek 官方一手资料作为外部事实依据：

- [DeepSeek-V4-Pro 正式版上线（2026-08-13）](https://api-docs.deepseek.com/zh-cn/news/news260813)
- [DeepSeek Responses API 兼容性明细](https://api-docs.deepseek.com/zh-cn/guides/responses_api)
- [DeepSeek 模型与能力规格](https://api-docs.deepseek.com/zh-cn/quick_start/pricing)

Phase D 的 PLD-006 采用当前 0.147 源码和以下一手资料校验扩展与迁移边界：

- [Codex 0.147 Extension Registry](https://github.com/openai/codex/blob/rust-v0.147.0/codex-rs/ext/extension-api/src/registry.rs)
- [Codex 0.147 Goal Extension](https://github.com/openai/codex/blob/rust-v0.147.0/codex-rs/ext/goal/src/extension.rs)
- [SQLx 0.9 `MigrateError`](https://docs.rs/sqlx/0.9.0/sqlx/migrate/enum.MigrateError.html)
- [SQLite ALTER TABLE 与安全 schema 迁移步骤](https://www.sqlite.org/lang_altertable.html)

| ID | Before Phase | Previous Plan | Current Fact | Proposed Change | Impact | User Approval | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PLD-001 | Phase A | 继续以 0.146 为候选；U2 验证 overlay；U3 替换 vendor | 0.147 已于 2026-08-07 发布，固定 commit 为 `be6e8eac029b183056b7e4402879f15d2c85f61b`；0.146 工件不能证明 0.147 | 当前候选改为 0.147；新增候选资格单元 U2；原后续单元顺延 | target-dependent 工件和资格证据必须重算；生产替换推迟到 U4 | `user-approved-plan-direct: “根据147正式更新计划”` | approved-applied |
| PLD-002 | Phase A | 无 phase rebase gate；U2 同时执行资格与全部工件重算；完整包测试未做环境能力预检 | 新版 `se-good-plan` 要求每个 material Phase 先重基；U1 已证明本机 sandbox/network 能力会制造同源噪声；0.147 新能力可能随整仓替换进入默认产品面 | 补齐五个 phase gate；U2 增加资格优先 checkpoint 和 sandbox preflight；U3 增加新能力默认暴露审计；校正 execution/validation 状态 | 不增加运行时架构；候选失败可更早停止；U4 前新增明确的权限/持久化/协议冲突门禁 | `user-approved-plan-direct: “根据审查结果先治理方案”` | approved-applied |
| PLD-003 | Phase B | U4 作为只含上游 substrate + U3 seam 的独立提交，随后才进入 DeepSeek/TaskSpace | cache gate 把旧 Whale final-wire/policy 的同批删除与 0.147 缓存敏感源码替换识别为硬冲突；当前无 DeepSeek/TaskSpace 的 U4 也不是有效真实回归主体 | 增加独立 U4a：在不改产品源码和 accepted baseline 的前提下治理 vendor-cutover 的 cache contract/提交边界；U4 重新通过免费门禁后再提交；真实 2-sample 回归推迟到 DeepSeek/TaskSpace 被测闭环恢复后 | 增加一个测试治理单元，但避免巨型 U4–U16 合并提交、无效付费运行或绕过门禁 | `user-approved-plan-direct: “批准”` | approved-applied |
| PLD-004 | Phase C | U6 保持 Pro 隐藏；U7 重放旧 Chat Completions 转换和 SSE 适配，再按原顺序完成 U8–U10 | DeepSeek 于 2026-08-13 发布 V4 Pro 正式版并原生支持 Responses API；官方兼容表明确 Flash/Pro 均支持 Responses；Codex 0.147 已移除 Chat Completions wire 分支并采用 Responses-only 主链 | 不恢复旧 Chat Completions 转换层；U5 基于当前 provider seam 恢复 DeepSeek 身份、鉴权与 Flash 默认；U7 只按官方兼容表补足确有测试证据的 Responses 请求/SSE 差异；U8–U10 完成后再执行 U6，使 Flash 继续默认、Pro 在 provider/final-wire 与 TUI 验证通过后恢复可见 | 执行顺序调整为 U5→U7→U8→U9→U10→U6；减少废弃兼容代码和上游侵入；D1 的官方发布条件已满足，本地验证条件仍保留；0 模型请求不变 | `user-approved-plan-direct: “批准”` | approved-applied |
| PLD-005 | Phase C / U9 | 恢复“Flash compact request”，模型目录整体留到 U6 | 历史 Whale compaction 实现实际由 Flash 主任务切到 Pro 生成 checkpoint；V4 Pro 现已正式发布且原生支持 Responses；0.147 必须先能解析 Pro 元数据，才能构造该压缩请求 | U9 恢复隐藏的 Flash/Pro 运行时元数据及 1M/755K 合同，Flash 主任务压缩时只替换采样模型为 Pro；目录选择器可见性仍留到 U6 | 修正旧计划中与历史实现不符的措辞；不提前暴露模型、不增加 TaskSpace 提示词/状态、不发送真实请求 | `user-approved-plan-direct after V4 Pro release reminder: “批准”` | approved-applied |
| PLD-006 | Phase D | U11–U14 把旧 `core/action_map`、state store、tool handler 和 session hooks 依次直接重放到 0.147，未单列旧数据库升级兼容 | 旧 TaskSpace 跨 177 个引用路径，直接回放会重新侵入 core/session/provider；0.147 已提供 tool、tool lifecycle、thread/turn lifecycle、world-state 和 event sink 扩展 seam；旧 Whale 与 0.147 对 `state/migrations/0030`、`0031` 使用了不同 SQL/checksum，现有旧库会先触发 SQLx `VersionMismatch` 并使 state runtime 不可用；上游 Goal extension 已证明 state-backed extension 模式可行 | U11 改为精确指纹保护的旧库 migration bridge；U12 只迁 canonical TaskSpace domain/event kernel 到独立 `ext/taskspace` crate；U13 在现有 `StateRuntime` 上恢复同一 TaskSpace store/CAS/replay，并以新迁移号兼容新旧库，不新增第二状态库；U14 通过现有 extension contributors 接入 tools、lifecycle 和 WorldState，除非 seam spike 证明缺口，否则禁止恢复旧 core/session/provider-wire 侵入；U15 通过 extension service 暴露 RPC/schema；U16 恢复 TUI/viewer 并完成 TaskSpace final-wire/cache 合同 | 增加一个必须先完成的数据兼容单元；把 tool/session 两个宿主侵入单元合并为 extension 集成边界；保持 TaskSpace canonical store 为唯一任务状态权威，AgentGraphStore 只管理 thread spawn topology，WorldState 只承载模型可见 projection；预计 Phase D 总生产改动明显超过 500 行，批准方向后仍按 U 单元控制范围，U12 开始前需给出精确移植清单与代码预算 | `user-approved-plan-direct: “按照你建议执行”` | approved-applied |
| PLD-007 | Phase F | 0.147 融合已经收口，后续更新未进入当前计划 | 官方已发布稳定版 0.148/0.149；0.147→0.149 变化 2,014 路径，其中 116 条与当前 290 路径 Whale overlay 重叠；只读三方 apply 预检约 30 个冲突点，集中在 core/app-server/TUI/schema/lockfile | 新增 U19：先把同步元数据与资格工具参数化到固定 0.149，再以官方 0.147→0.149 差分三方应用；分别闭合通用 substrate、DeepSeek/cache、TaskSpace/app-server/TUI，最后刷新生成物和 provenance | 不新增同步框架或双 vendor；机械上游代码不计 Whale 手写生产代码预算；任何需要改变 D1/D2 的上游默认行为必须停止并请示；真实模型验证不自动继承既有预算 | `user-approved-plan-direct: “追赶到149”` | approved-applied |

## 4. 最低成本预投资验证

| ID | Critical Assumption | Decision Unlocked | Cheapest Credible Method | Enough Evidence / Not Proven | Budget / Isolation | Stop / Cleanup | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| V1 | 0.146 的四项失败是否主要来自错误命令、ambient proxy 或过严 `--locked` | 是否继续以 0.146 为候选 | 在临时官方树读取该 tag 自带 README/justfile/CI，清空非必要代理和 Whale 环境，执行官方入口 | 三个 runner 问题已证实并修正；完整 core/app-server/TUI 入口未全部完成 | 独立临时树和 target；0 模型请求 | 日志已保留；0.146 不再作为当前候选 | direction-rejected |
| V2 | 0.147 是否在复用 U1 runner 修正后具备进入 overlay 验证的资格 | 是否执行 U2 checkpoint B 和 U3 | 固定官方 commit；探测宿主能力；按候选自带 `setup-rusty-v8` 合同从 OpenAI 专用 release 下载并校验 archive/binding；再执行六项矩阵 | 官方资产、helper、CLI、app-server 已通过；core/TUI 剩余失败已归为硬编码 `/tmp`、MCP 时序与 release snapshot 风险 | 独立临时树和 target；0 模型请求；不写生产 vendor | 保留历次 evidence；风险进入后续回归归因，不修改候选规避 | direction-supported-with-known-test-risks |
| V3 | 0.147 substrate 能以很薄的 identity/home overlay 支撑 Whale CLI | 是否替换生产 vendor | 一次性临时候选树只应用品牌、二进制身份、`WHALE_HOME`、auth 隔离 patch | CLI build/version、home、direct keyring 与 encrypted secrets keyring 隔离均通过；不需要 DeepSeek/TaskSpace stub | 未提交第二份 vendor；0 模型请求 | 临时树已删除；证据落入 U3 report | validated |
| V4 | 0.147 新增用户可见能力不会在整仓替换时静默改变 Whale 默认权限、持久化或协议行为 | 是否执行 U4 | 在临时 0.147 tree 检查 `--approve-for-me`、portable Agent Plugins、thread sections、MCP 2026-07-28 的 CLI help、配置 schema、feature/default、protocol 和持久化入口 | approve flag 与 thread RPC 为显式动作；MCP 2026 默认 false；remote plugin/sharing 的上游默认 true 已通过现有 feature seam 锁回 false | 只读源码 + 本地无模型 smoke；不改生产候选 | 未新增禁用框架；临时树已删除；证据落入 U3 report | validated |
| V5 | Phase C 是否仍需旧 DeepSeek Chat Completions 转换层，Pro 是否仍应隐藏 | 是否按原 U6/U7 方案执行 | 核验 DeepSeek 官方正式版公告、Responses 兼容表、模型规格，并对照 Codex 0.147 provider/endpoint 源码 | 官方确认 V4 Pro 正式版与 Flash/Pro 原生 Responses 支持；0.147 为 Responses-only；足以否定旧转换层方向，但不能替代本地 provider/final-wire/TUI 回归 | 只读官方资料与本地源码；0 模型请求 | PLD-004 已批准；保留 D1 的 Flash 默认和本地验证门槛 | direction-supported |
| V6 | 旧 TaskSpace 是否能按原 U11–U14 直接重放，且旧 Whale state DB 能直接由 0.147 打开 | 是否保持 Phase D 模块边界和顺序 | 只读比较切换前 TaskSpace 引用面、旧/新 migration SQL+checksum、0.147 extension/state/AgentGraph/WorldState 源码与 SQLx 0.9 校验语义 | 旧实现跨 177 个引用路径；extension API 已覆盖主要宿主 seam；旧/新 0030、0031 checksum 不同，SQLx 对已应用但内容变化的同版本返回 `VersionMismatch`；足以否定直接重放顺序，但尚未证明 migration bridge 的最终 SQL 和 canonical kernel 精确移植清单 | 只读 Git 对象、当前源码和官方文档；0 模型请求；未读取任何其他工作空间或用户数据库 | PLD-006 已批准；U11 先用合成旧库 fixture 验证，未知 checksum 必须 fail-closed | validated |
| V7 | 0.149 能否沿用 0.147 overlay 而不重建 Whale 业务层 | 是否进入 U19 vendor cutover | 固定官方 tag/commit；比较官方差分与当前 overlay；执行只读三方 apply check | 官方 tag 身份已核验；2,014 个变化路径、116 个 overlay 重叠、约 30 个冲突点，证明可沿现有 seam 语义重放，但不能把自动合并等同于行为通过 | 当前仓库对象库与只读 index check；0 模型请求；无第二 worktree | 进入 U19；冲突若要求改变 D1/D2 或建立双状态权威则立即停止 | direction-supported |

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
- Gate status：`ready`

进入条件：Phase A verified，用户已看到 0.147 资格结论。适用决策：D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U4 | 替换 vendor substrate | dependency/source | `third_party/codex-cli/`、`UPSTREAM.md` | 已验证的 0.147 源码与 workspace metadata | 已用官方 Git archive 替换 vendor，只重放 U3 patch；保留官方锁文件 | vendor 对齐 0.147，Whale 数据仍隔离；DeepSeek/TaskSpace 未混入本单元 | 建立可维护 substrate，减少逐补丁漂移 | Complexity：大规模机械变化、无新业务抽象；Reach/Cost：全 workspace 构建和审查成本高 | SHA/tree/license、U3 tests、offline CLI build/smoke、42 个 metadata 单测、cache index gate 通过 | 独立 revert U4；产品 overlay 仍按 U5–U16 分单元恢复 | verified |
| U4a | 迁移 vendor cutover 缓存验证合同 | test/governance | cache gate contract、free final-wire fixtures、U4 提交边界 | 0.147 已移除的 Whale 专用 cache policy/final-wire 合同 | 已在不修改产品源码、accepted baseline 或真实回归结果的前提下迁移免费合同 | U4 的缓存敏感变化可被免费证据发现并与后续真实回归边界分离 | 保留强制门禁，同时避免无被测链路的付费运行 | Complexity：限于测试/门禁；Reach/Cost：增加免费测试维护成本，0 模型请求 | 53 个 gate/contract 单测；free final-wire 全矩阵；index gate 通过；accepted baseline 未变 | 独立提交 `17424eac8` 可回滚；不得据此晋升 live baseline | verified |

退出条件：已满足。U4 常规验证与 cache index gate 均通过；Phase B 未重放 DeepSeek/TaskSpace，也未改变 accepted live baseline。证据见 [U4 vendor cutover report](../../migration/codex-sync/2026-08-13-u4-vendor-substrate-cutover.md)。

### Phase C：DeepSeek 闭环

#### Pre-Phase Plan Rebase Gate

- Rebase scope：U4 实际 vendor substrate、编译缺口、上游 provider/catalog/Responses/cache seam + Phase C–E 剩余计划。
- Material plan delta：`material`
- Plan delta record：PLD-004、PLD-005
- User approval：`user-approved-plan-direct: “批准”`
- Gate status：`ready`

进入条件：U4 verified。适用决策：D1、D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U5 | 恢复 provider 身份与鉴权 | provider | `model-provider-info` 及配置调用点 | DeepSeek provider identity、base URL、API key auth、Flash 默认 | 已按 0.147 provider 接口恢复内置 DeepSeek provider、保留 `DEEPSEEK_API_KEY` 环境鉴权，并把无显式配置时的 provider/model 恢复为 DeepSeek/Flash；未引入模型目录与请求 payload | CLI 能识别、配置并默认选择 DeepSeek provider 与 Flash | 先恢复最小接入点，隔离目录和 wire 问题 | Complexity：一个局部 provider 分支和配置默认值；Reach/Cost：配置解析和 auth tests | provider 23、config 235、core config 307 项通过；cache index gate 通过；0 真实请求 | 独立 revert；模型目录与能力留给 U6 | verified |
| U7 | 恢复原生 Responses 请求与流式事件 | provider-wire | `codex-api` 的 Responses/SSE seam | DeepSeek Responses request、reasoning stream、tool-call assembly | 已用显式 DeepSeek provider 请求和官方形状 SSE fixture 验证 0.147 Responses-only 主链；fixture 直接通过，因此未增加生产分支，也未恢复 Chat Completions 转换层 | reasoning、文本和完整 function call 可由 DeepSeek 原生 Responses 进入现有事件链 | 以零生产侵入恢复主请求合同并淘汰失效兼容假设 | Complexity：仅 2 组测试 fixture；Reach/Cost：无 runtime 变化、0 网络请求 | fmt；2 个定向 fixture；codex-api 159 项；42 个 sync tests、metadata 和 cache index gate 全过 | 测试可独立回滚；final-wire/cache 仍留给 U10 | verified |
| U8 | 恢复用量与开发期请求保护 | accounting | `core/src/client.rs` 现有 transport/accounting seam；Realtime start seam | provider usage、开发期 hard request limit、terminal reconciliation | 已将显式环境变量触发的 transport-exact guard 接到 HTTP/WS/compact/memory/realtime dispatch；跨 client 共享计数、关闭隐藏重试并对无法精确计数的 Realtime fail-closed；completed usage 继续由上游事件链写入 rollout terminal | 普通产品运行不变；获批真实回归可获得机械请求上限；provider usage 与终止记录一致 | 防止主线融合丢失成本保护，同时不恢复 TaskSpace budget 状态或产品授权协议 | Complexity：一个默认关闭的局部 guard，无第二 accounting 状态；Reach/Cost：client/realtime tests，0 网络请求 | fmt；5 个 hard-limit、20 个 client、15 个 realtime tests；usage terminal fixture；sync/cache gate | 独立 revert；不修改 compaction threshold、TaskSpace 或 ledger 规则 | verified |
| U9 | 恢复 DeepSeek compaction | context-lifecycle | `core/src/compact.rs`、context-window 与隐藏模型元数据 | Flash 主任务的 Pro compact request、1M/755K threshold、上游 checkpoint 状态保留 | 已按 PLD-005 在既有 local compaction 路径只替换 DeepSeek 压缩采样模型；恢复 Flash/Pro 隐藏运行时元数据；复用上游 checkpoint prompt/history replacement，不引入 TaskSpace projection | 短作业不压缩；到 755K 触发；Flash 压缩请求使用 Pro；模型仍不在选择器显示 | 将上下文生命周期风险从 wire/cache 中隔离，并避免恢复旧专用状态提示词 | Complexity：一个局部采样模型选择和目录数据；Reach/Cost：context/session/mock tests，0 真实请求 | model catalog、sampling、threshold/zero-short-job、mock final request、compact regression；sync/cache gate | 独立 revert；未修改 TaskSpace retention，选择器可见性仍由 U6 控制 | verified |
| U10 | 恢复缓存与 final-wire 证据 | cache/observability | DeepSeek mock final-wire、既有 cache contract/gate | Standard 最终 payload 与敏感面 | 已在 U9 mock 链补齐 Standard final-wire 精确断言，并运行现有五组免费缓存合同和 index gate；旧 `provider_wire_trace` 强耦合 TaskSpace，未在本单元恢复 | 零模型请求即可锁定 Standard 请求字段与通用前缀/cache-key/MCP/API 合同 | 付费回归前提供确定性保护，不提前引入 TaskSpace 观测代码 | Complexity：只增加测试断言和证据文档，0 runtime state；Reach/Cost：cache-sensitive test gate，0 真实请求 | Standard final-wire；5 组 free contracts；sync/metadata；cache index gate | 独立 revert；live baseline 保持失败且未晋升；TaskSpace trace 留给 Phase D | verified |
| U6 | 恢复模型目录与可见性 | catalog/defaults | `models-manager` 及 model selector | Flash/Pro catalog entries、default/visibility | 已在 U5、U7–U10 验证后把 Flash/Pro 目录项恢复为可见；复用上游 `show_in_picker` 选择器，只增加 DeepSeek-only 公共列表和 Flash 默认规则，并更新 TUI 快照 | 公共模型列表只展示 DeepSeek；Flash 无论远端优先级如何仍为默认，正式版 Pro 可供用户选择 | 在 provider/final-wire 已验证后开放 Pro，不建立 Whale 专用 UI 分支 | Complexity：两个局部目录策略函数和现有 selector；Reach/Cost：model manager tests 与 1 个 TUI 快照，0 真实请求 | model manager 50 项；TUI picker 定向测试；sync/metadata/cache index gate；D1 检查 | 独立 revert U6 可重新隐藏 Pro，不回滚已验证 provider 主链 | verified |

退出条件：已满足。U5–U10 全部 verified；D1 的 Flash 默认和正式版 Pro 可见语义 covered；没有 TaskSpace 产品语义变化。证据见 U5–U10 各单元报告，U6 收口报告为 [DeepSeek 模型目录与可见性](../../migration/codex-sync/2026-08-14-u6-deepseek-model-catalog.md)。

### Phase D：TaskSpace 闭环

#### Pre-Phase Plan Rebase Gate

- Rebase scope：U4 substrate 与 U5–U10 DeepSeek 实现、TaskSpace 实际编译/测试状态、上游 domain/data/tool/session/client seam + Phase D–E 剩余计划。
- Material plan delta：`material`
- Plan delta record：PLD-006
- User approval：`user-approved-plan-direct: “按照你建议执行”`
- Gate status：`ready`

重基结论：PLD-006 已批准，U11–U16 按以下替换映射执行：

1. U11 先解决旧 Whale `0030/0031` 与 0.147 migration 的同号 checksum 冲突；只识别已知旧 checksum，保留 TaskSpace 表与数据，未知历史 fail-closed。
2. U12 只恢复 canonical map v2、DAG invariant、domain event/transaction/serialization 到独立 `ext/taskspace`，不搬运旧兼容层、provider trace 或宿主路由。
3. U13 复用现有 `StateRuntime` 恢复 TaskSpace store、CAS 与 replay；用新的迁移号覆盖 fresh 0.147 和经 U11 修复的旧 Whale 库，不创建第二 TaskSpace 状态库。
4. U14 复用 0.147 extension registry 接入 native tools、tool lifecycle、thread/turn lifecycle、WorldState projection 和 extension event sink；AgentGraphStore 只作为 thread spawn topology，不成为 TaskSpace 状态权威。
5. U15 通过 extension-owned service 恢复 app-server RPC、事件与生成 schema，避免给 `CodexThread` 重新增加 TaskSpace 专用方法。
6. U16 恢复 `/taskspace`、`/task-show` 与 viewer，并在同单元锁定 TaskSpace final-wire/cache；不恢复强耦合旧 `provider_wire_trace`。

规模门禁：PLD-006 的方向批准不等于无限制代码授权。U11 目标控制在 500 行手写生产代码以内；U12 开始前必须先落精确保留/淘汰清单与预估生产代码量，若超过 500 行须取得用户对该工作单元的明确批准。

进入条件：Phase C verified。适用决策：D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U11 | 建立旧 Whale state DB 迁移桥 | data-compatibility | `state/src/migrations.rs`、migration tests | 已应用旧 TaskSpace `0030/0031` 的 `_sqlx_migrations` 与表数据 | 已在正常 migrator 前精确识别两条已知旧 checksum；事务补执行 0.147 的同号 schema 动作并把 migration metadata 更新为当前 checksum，保留 TaskSpace 表和数据；未知或部分匹配不改写 | 已知旧 Whale 数据库可继续由 0.147 打开并保留 TaskSpace 数据；fresh 0.147 与未知数据库行为不变 | 先解除真实升级阻塞，避免后续功能恢复建立在不可启动的旧库上 | Complexity：99 行局部生产改动，不增加新数据库/配置；Reach/Cost：仅 state 初始化与 migration tests，0 真实请求 | 3 个迁移桥测试通过；`codex-state` 173 passed；合成旧库保留 canonical JSON；fresh/current no-op；单条/未知/部分 schema fail-closed | 独立 revert；未触碰真实用户 DB；任何未知 migration 形态仍由 SQLx 阻断；证据见 U11 报告 | verified |
| U12 | 恢复 TaskSpace canonical kernel | domain | 新 `ext/taskspace` crate 的 model/invariant/transition/event/transaction 模块 | canonical map v2、DAG invariant、domain transaction/event/serialization | 已按获批清单完成 U12a–c：只迁 Rooted DAG kernel 与 strict canonical types；排除 store/session/provider/tool/RPC/TUI/projection | TaskSpace 业务状态与转换规则可独立构建和重放 | 固定业务语义，并把旧 `core/action_map` 宿主侵入收敛为独立扩展内核 | Complexity：实际 1,691 行生产代码，低于 1,870 行硬上限且单文件小于 500；Reach/Cost：独立 crate，0 真实请求，无 host crate 依赖 | 34 tests；256-case property、event/terminal/reopen replay、serialization fixtures；Clippy `-D warnings` passed | 独立 revert U12a–c；U13 前不接 state/session/provider；证据见 U12 报告 | verified |
| U13 | 恢复唯一 TaskSpace store 与 replay | persistence | `state` 的 `0047` migration 与 TaskSpace store adapter | canonical store、CAS commit、thread binding、replay | 已复用现有 `StateRuntime` pool；fresh/current DB 由 `0047` 创建三张表，经 U11 修复的旧表原地复用；写入/读取调用 U12 canonical validation；commit ID 提供幂等 replay；CAS 同时约束 owner 不变与 domain revision 单调递增 | canonical TaskSpace 状态可持久化、迁移和确定性重放，旧数据不复制到第二权威 | 保持一套 TaskSpace 状态权威，同时利用上游 state 生命周期 | Complexity：441 行生产改动、单 store adapter、不建独立 DB；Reach/Cost：state/runtime、并发 CAS、升级矩阵，0 请求 | 7 focused；state 177 passed；Clippy clean；fresh/legacy、CAS conflict、owner/binding、event replay、data preservation | 独立 revert；AgentGraphStore/WorldState 未接入；证据见 U13 报告 | verified |
| U14 | 通过 0.147 extension seam 接回 TaskSpace runtime | extension-integration | `ext/taskspace` contributors 与 core response dispatch 汇聚点 | native control tools、tool batch/lifecycle、thread/turn lifecycle、WorldState | 五段完成 read/rehydrate、response preflight、active Map `execute`、显式启用/初始化及 terminal/reopen | 跨线程 runtime、canonical projection 与完整写循环均已恢复；extension sink 只接受已定义的 `EventMsg`，故事件定义/发射与 U15 schema 同单元完成，避免半套 wire | 以窄 extension seam 取代旧 handler/sequence/session/provider-wire 分支，canonical store 保持唯一写权威 | Complexity：五段分别为 438、229、494、321、249 行生产新增，均按原子段受控且单文件小于 500；Reach/Cost：response 批次、state CAS、tool lifecycle、mode gate 与 terminal history，0 请求 | extension registry 6、taskspace 40、state CAS 4、core zero-dispatch 1 passed；TaskSpace Clippy/fmt clean；finish sibling 拒绝零提交，close→reopen→release 端到端通过 | 五段可独立回滚；事件进入 U15 versioned wire 单元，RPC/TUI 仍留 U15/U16 | verified |
| U15 | 恢复 app-server TaskSpace API | API | app-server protocol/source schema、`ext/taskspace` service、app-server adapter | read/mode RPC、TaskSpace events、JSON/TS schema | 三段完成 service refresh/read、兼容 method `thread/mapRuntimeMode/set` 与 `thread/taskspace/read`、版本化 `thread/taskspace/updated` notification；事件只在更高 revision 成功持久化后发射，并生成 JSON/TS/precomputed exports | 已加载线程可显式启用/关闭、读取 TaskSpace，并按 canonical revision 收到有序失效通知；Standard 默认不变；完整状态仍由 read RPC 返回 | 隔离协议兼容面，不把 RPC 再耦合进 core session；通知保持轻量，避免复制 canonical Map 或建立第二状态权威 | Complexity：service seam、约 390 行 RPC 生产代码、约 180 行 event 生产代码及机械 schema；Reach/Cost：protocol、extension sink、app-server listener、schema，0 请求 | TaskSpace 40、protocol wire 1、event FIFO 1、schema fixtures 6 passed；三 crate all-target Clippy clean | 三段可独立回滚；TUI/viewer/final-wire 保留 U16，不增加 TUI fallback 或第二 runtime | verified |
| U16 | 恢复 TUI、viewer 与 TaskSpace wire/cache 闭环 | client/cache | TUI slash routing/viewer、TaskSpace mock final-wire、cache contracts | `/taskspace`、`/task-show`、viewer、projection payload | 三段完成 typed RPC 路由、localhost canonical viewer，以及同一 mock 线程内 Standard→TaskSpace 最终 Responses body 与免费缓存合同 | 用户可显式进入并查看实时 TaskSpace；TaskSpace 只增加扩展工具和动态 WorldState，公共工具、conversation prefix、`instructions` 与 cache key 保持稳定 | 以独立 222 行 viewer 取代旧派生页面；复用 core mock 与既有 cache contract，不恢复 `provider_wire_trace` 或运行时观测层 | Complexity：第一段 113 行、第二段约 226 行生产改动、第三段仅测试/门禁；Reach/Cost：TUI、typed client、localhost HTTP、final-wire，0 请求 | slash 3、viewer 2、DeepSeek final-wire 2、cache Python 20、六组免费合同、core tests Clippy passed | 三段可独立回滚；既有夹具与 Windows 继续延期；live baseline 未晋升 | verified |

退出条件：已满足。U11–U16 verified；canonical store 保持唯一状态权威；TaskSpace TUI 已知夹具和 Windows 验证未新增回归且继续明确延期。证据见 U11–U16 各单元报告，U16 收口报告为 [TaskSpace final-wire 与免费缓存合同](../../migration/codex-sync/2026-08-14-u16-taskspace-final-wire-cache.md)。

### Phase E：发布收口

#### Pre-Phase Plan Rebase Gate

- Rebase scope：U4–U16 实际实现、生成物、Linux/Windows 延期、缓存门禁和剩余发布风险 + Phase E 计划。
- Material plan delta：`none`
- Plan delta record：`none`
- User approval：`not-required`
- Gate status：`ready`

重基结论：U4–U16 的实现未改变 U17 的发布收口目标，但将执行边界收窄为纯核验与文档同步：

1. 更新已停留在 U4 时点的 `third_party/codex-cli/UPSTREAM.md`，记录当前 0.147 substrate、DeepSeek 与 TaskSpace overlay；来源 commit/tag 不变。
2. 刷新并验证 overlay inventory/replay ledger、schema 与 lockfile；不复制或改写 vendor 内的上游 CI/release workflow，也不新增 Whale 发布框架。
3. Linux 使用 0.147 官方 Cargo 入口执行 fmt、workspace all-target check、workspace test 和 CLI build/smoke；失败必须按实际签名归属，不为追求全绿修改无关模块。
4. 执行完整免费缓存合同和 index gate；accepted live baseline 保持最近一次失败状态，U17 不申请真实模型预算、不晋升 baseline。
5. TaskSpace TUI 已登记夹具以及 Windows runner/终端 smoke 继续明确延期；没有对应平台证据时不声明通过。

进入条件：Phase D verified。适用决策：D1、D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U17 | 完成发布闭环 | release | workspace metadata、`UPSTREAM.md`、最终 migration report | 来源、生成物、Linux 支持矩阵 | 已更新 provenance 文档，刷新并验证机械工件，完成 Linux Cargo/CLI/缓存矩阵；对抗性修复补齐旧空 map、普通 fork lineage、生产组合测试与逐失败审计 | 已形成可追溯的 0.147 substrate、185 路径 Whale overlay 与真实通过/延期清单 | 防止局部通过误报为整体完成 | Complexity：收口修复仅扩展既有 state/lifecycle seam，无新框架；Reach/Cost：全量 app-server/core 矩阵与逐项日志，0 模型请求 | 原有 fmt/all-target/CLI/schema/cache/exec/MCP 证据保持；精确提交 app-server 1089/1122、core lib 2154/2178、core integration 1086/1123；94 个失败逐项映射，代理环境项隔离复跑 1/1；production app-server/SQLite/mode-read/fork/restart/final-wire 定向测试 1/1 | TUI/Windows、OpenAI hosted/Bedrock/Guardian、V8 非 sandbox 制品继续延期；live baseline 未晋升 | verified |
| U18 | 将已完成的 0.147 融合重放到最新 `main` | integration/rebase | 当前分支、0.147 vendor overlay、state migration、TaskSpace production composition、同步元数据 | rebase 冲突与 rebase 后行为等价性 | 已把 119 个本地提交重放到 `main@df5da4d3944448a9ae877d601f8c8045c415d983`；按 0.147 当前模块边界重新接入 R8 TaskSpace，保留 `0047` 历史 checksum，并用 `0048` 无损归档旧 v2 JSON 表后创建 relational store；补齐 Standard→mode/read RPC→TaskSpace exec→fork→shutdown/reopen/resume→final Responses wire 的生产组合测试；修复 0.147 扩展 schema 的小数 `minimum` 兼容和 OpenAI 专用测试夹具；稳定测试夹具中的临时身份后完成受控 cache baseline 晋升 | 当前分支以最新 main 为 Git 基座，DeepSeek 默认与 TaskSpace 语义保持；旧 v2 数据保留但不会被猜测性激活；生产组合链可跨 fork/restart 恢复；当前敏感面已有与同一 HEAD 对齐的 accepted baseline | 消除文本冲突之外的 schema、迁移号和运行时组合冲突，不引入第二状态权威或新业务框架 | Complexity：主要为 rebase 语义迁移、既有 R8 代码适配和测试；Reach/Cost：state/core/tools/app-server；真实资格运行仅覆盖 Standard + map-request 两个 sample run，实际 `0.01973628 CNY` | state 190、tools 106、TaskSpace core 72、image ext 10、web ext 8、production composition 1 均通过；4,881 项隔离矩阵完成；最新 index cache gate PASS；缓存 Python 232/232、Standard/TaskSpace final-wire 各 1/1；overlay/replay 290 路径且 metadata validation PASS；剩余失败按既有 hosted/provider/platform 清单延期 | 精确 migration checksum 可回滚；旧 v2 表保留为 archive；OpenAI hosted/Bedrock/Guardian、Windows/TUI 不在本单元扩张 | verified |

退出条件：U17、U18 verified；所有修改已提交并 push；工作树 clean；延期项未被表述为通过。U18 的代码、真实缓存资格、baseline 晋升和本地门禁已完成，当前只剩提交与精确 lease 推送。

### Phase F：0.149 稳定版追赶

#### Pre-Phase Plan Rebase Gate

- Rebase scope：U18 已验证实现、当前 290 路径 overlay、0.149 官方 tag、0.147→0.149 差分与只读三方 apply 预检。
- Material plan delta：`material`
- Plan delta record：`PLD-007`
- User approval：`user-approved-plan-direct: “追赶到149”`
- Gate status：`ready`

重基结论：继续使用现有唯一 vendor 和现有 DeepSeek/TaskSpace seam。先应用官方稳定版差分，再只处理实际冲突；不恢复已淘汰的旧 provider/session 分支，不启用 OpenAI hosted、Bedrock、Guardian 或 remote plugin 产品面。生成 schema/lockfile 从最终源重新生成，不手工长期维护冲突结果。

进入条件：U18 verified、工作区门禁 ready、固定 0.149 commit 已核验。适用决策：D1、D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U19a | 固定 0.149 候选与同步合同 | qualification/metadata | `scripts/codex-upstream/`、candidate/delta metadata | release identity、generator constants、schema validation | 将固定 tag/commit/V8 合同更新到 0.149，生成 0.147→0.149 差异证据并执行最低成本官方入口验证 | 后续 cutover 以可追溯且可复验的官方对象为输入 | Complexity：只改既有同步脚本常量与测试；Reach/Cost：metadata 与本地构建，0 模型请求 | 脚本单测、candidate identity、fmt/CLI check；完整重型资格可按收益分层执行 | 若候选基础构建方向不支持则停止，不改生产 vendor | in-progress |
| U19b | 应用 0.149 substrate 并解决通用冲突 | vendor/substrate | `third_party/codex-cli/` | 官方 0.147→0.149 差分、Whale identity/home/auth/build seam | 以三方 apply 应用完整官方差分；通用文件优先采用上游，保留最小 Whale identity/home/auth 与已确认 feature 默认值 | vendor substrate 升至 0.149，同时不混入 DeepSeek/TaskSpace 产品重构 | Complexity：约 2,014 路径机械变化，实际人工冲突局限于预检热点；Reach/Cost：workspace build、CLI、权限与持久化 | 冲突清零、fmt、CLI check、identity/home/auth/feature 定向测试 | 单提交可 revert；若出现产品决策冲突停在未提交状态并请示 | not-started |
| U19c | 重放 DeepSeek 与缓存合同 | provider/cache | provider/model/core Responses 与 cache fixtures | DeepSeek-only catalog、Flash 默认、Pro、1M/755K、usage/final-wire | 在 0.149 provider/model/compaction seam 上保留 D1，并重新生成及验证 Standard final-wire；不恢复旧 Chat Completions 层 | DeepSeek 主路径保持原生 Responses 与既有模型语义 | Complexity：只适配实际 API 变化；Reach/Cost：provider、model catalog、compaction、cache；默认 0 模型请求 | provider/catalog/compaction/SSE/usage 定向测试、cache Python suite、index gate | 若 cache gate 要求新真实证据，先停下按预算规则申请 | not-started |
| U19d | 重放 TaskSpace 组合链并发布收口 | taskspace/release | extension/state/core/app-server/TUI、generated schemas、provenance | relational store、lifecycle、RPC/events、fork/resume/final-wire | 保持唯一 TaskSpace relational store与 extension seam，适配 0.149 history/permission/multi-agent 变化；重生成 schema/lockfile，刷新 inventory/replay/UPSTREAM 和收口报告 | TaskSpace 在 0.149 上继续支持 mode/read/exec/fork/restart/final-wire，官方新能力不改变 Whale 默认产品面 | Complexity：不建立第二状态权威或兼容框架；Reach/Cost：state/core/app-server/TUI 与生成物，Windows/hosted 延期不改判 | state/TaskSpace/app-server 组合测试、隔离相关回归、cache gate、metadata validator、CLI smoke | 代码和生成物按原子提交可回滚；真实模型与 Windows 验证需独立授权/环境 | not-started |

退出条件：U19a–U19d verified；vendor provenance 指向 0.149；D1/D2 无冲突；所有修改已提交并 push；工作树 clean；延期项未被误报为通过。

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
| Phase C | 已完成 | PLD-004/005 已批准；U5–U10 已恢复 provider、原生 Responses、开发期 guard、1M/755K 与 Flash→Pro 压缩、Standard final-wire/cache；U6 使 Flash/Pro 可见、公共列表仅保留 DeepSeek，并保持 Flash 默认；未触及 TaskSpace | D1、D2 | covered + engineering-only | Phase D rebase 与 PLD-006 审批已完成 |
| Phase D | 已完成 | U11–U16 verified；旧库 bridge、canonical kernel/store、extension runtime、RPC/event/schema、TUI/viewer 与免费 final-wire/cache 均已闭环；Standard 默认路径不变 | D2 | covered + engineering-only | 进入 Phase E rebase gate |
| Phase E | 已完成 | U17 已刷新 provenance/overlay/replay，完成 Linux 构建、离线 CLI、schema、免费 cache 与分层测试；非全绿项均保留真实签名和延期边界 | D1、D2 | covered + engineering-only | 主线 0.147 融合计划收口；后续延期项进入各自产品/平台单元 |
| Phase E / main rebase follow-up | 已完成 | U18 已将全部 0.147 融合提交重放到 `main@df5da4d`，以无损 archive 处理旧 TaskSpace v2 数据，以 relational R8 store 和生产 app-server final-wire 链重新锁定行为；专用 Standard + map-request 双臂资格运行成功，稳定 baseline 已晋升且 index gate 通过 | D1、D2 | covered + engineering-only | 提交并按精确 lease 推送；延期产品面维持原边界 |
| Phase F | 进行中 | 固定 0.149 为新稳定目标；沿用当前 DeepSeek/TaskSpace 产品语义，以官方差分三方应用并按实际冲突适配 | D1、D2 | covered + engineering-only | 完成 U19a–U19d；出现产品语义冲突时暂停请示 |

## 7. Pending Product Decisions

当前没有新的产品决策；U2 仅修正工程验证方法。以下情况出现时必须停下请示：

- upstream AgentGraph/WorldState 与 TaskSpace Event Store 无法保持单一任务状态权威；
- 后续拟偏离候选自带的官方 release/CI 构建合同，以本地源码构建或替换依赖规避真实发布缺口；
- DeepSeek 官方 Pro Responses 支持未来若发生回退或兼容性变化，需要重新审查 D1；
- 新上游能力会改变默认权限、持久化、模型可见性或用户控制方式。

## 8. 执行与提交边界

执行顺序已到达 `... -> U18 最新 main rebase 与 cache baseline qualification（已验证） -> U19 0.149 稳定版追赶（进行中）`。本轮主线融合仍以这一份文档为唯一计划；不得恢复旧 core/session/provider 专用分支或 `provider_wire_trace`。U19 默认只使用免费本地验证；若缓存敏感面要求新的真实回归，必须另走账本与预算授权。详细证据见 [U17 发布收口报告](../../migration/codex-sync/2026-08-14-u17-release-closeout.md) 与 [U18 main rebase 报告](../../migration/codex-sync/2026-08-21-u18-main-rebase-r8-semantic-migration.md)。

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
