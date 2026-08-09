# Codex CLI 主线融合执行计划

- 文档状态：有效，待执行
- Plan Validity：`valid-with-qualifications`
- 适用版本：WhaleCode v0.0.5
- 工作空间：仅 `/home/zhangxu/whalecode-codex`
- Product Authority：[./decisions.md](decisions.md)
- Applicable Decisions：D1、D2
- 当前生产 vendor：保持不变，直至 U3 通过自身门禁
- 当前候选证据：Codex CLI `rust-v0.146.0` qualification 为 no-go，须先按 U1 复核

## 1. 执行合同

- `decisions.md` 是本专题唯一产品决策权威源；active 决策只能由用户显式修改，Agent 禁止自我批准。
- 已验证的代码、测试、日志和上游事实可以修订本计划，但不能静默改写产品权威。
- 新出现的实质产品选择必须延期、局部 provisional，或交由用户确认；不得把工程实现倒推成产品决定。
- 每个实质阶段结束后，只审计该阶段的 Product Decision Delta，分类为 `covered`、`engineering-only`、`provisional` 或 `conflict`。
- 存在未解决的 material `provisional` 或 `conflict` 时，不得进入依赖它的下一阶段。
- 每个工作单元独立提交并 push；生成物必须与其权威源在同一单元生成和验证。
- 单元开始触及另一个产品域时立即停止并拆分，不通过新框架、双实现或临时业务分支维持进度。

## 2. 当前事实与治理结论

### 2.1 已完成事实

- 第一批 6 个独立安全/通用 backport 已合入并验证。
- 第二批以 12/15 verified、3/15 deferred 收口；W9 是 TaskSpace TUI 已知失败，W12/W13 是 Windows 验证延期。
- 第三批没有替换生产 vendor；0.146 纯上游资格矩阵为 1 passed / 4 failed。
- 现有 overlay inventory、candidate manifest 和测试日志仍是有效查询证据。

### 2.2 治理后的权威关系

| 工件 | 治理后角色 | 可以证明 | 不可以决定 |
| --- | --- | --- | --- |
| `backport-ledger.json` | 已完成变更证据 | 哪些独立补丁已合入 | 后续架构迁移顺序 |
| `upstream-candidate.json` 与 qualification 日志 | 候选事实证据 | 当时的候选身份和命令结果 | 失败一定属于上游产品缺陷 |
| `overlay-inventory.json`、`upstream-delta-inventory.json` | 路径查询索引 | 路径、hash、双方是否变化 | 文件的产品语义或处理方式 |
| `overlay-replay-ledger.json` | 非权威路由提示 | 自动分类结果和待人工检查热点 | `adapt`、`drop`、owner 或 cutover 的最终决定 |
| 第一至第三批计划/报告 | 历史执行记录 | 当时做过什么、得到什么证据 | 当前或未来执行授权 |
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

1. 证明固定候选可按其官方入口在隔离环境中构建和测试。
2. 以尽量接近上游原样的 vendor 为 substrate，只保留有产品依据的 Whale overlay。
3. 将 DeepSeek、缓存合同和 TaskSpace 分别作为独立闭环重放、验证和回滚。
4. 最终形成来源可追溯、行为可验证、下一次可重复的上游同步结果。

### 3.2 非目标

- 不重新设计 TaskSpace、Multi-Agent、Create/Debug Primitive 或模型分层。
- 不启用 Apps、Plugins、remote Code Mode、audio/image/realtime 等新增产品能力。
- 不主动修复 W9，也不把 W12/W13 Windows 延期项表述为通过。
- 不访问、检查或管理其他分支和工作空间。
- 不进行真实 Whale Agent run；缓存门禁若要求真实回归，另按预算规则申请。

### 3.3 最小充分路径

```text
官方候选资格复核
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

## 4. 最低成本预投资验证

| ID | Critical Assumption | Decision Unlocked | Cheapest Credible Method | Enough Evidence / Not Proven | Budget / Isolation | Stop / Cleanup | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| V1 | 0.146 的四项失败是否主要来自错误命令、ambient proxy 或过严 `--locked` | 是否继续以 0.146 为候选 | 在临时官方树读取该 tag 自带 README/justfile/CI，清空非必要代理和 Whale 环境，执行官方入口 | Enough：CLI/core/app-server/TUI 官方无模型入口可重复完成；Not proven：修改候选源码或弱化测试才通过 | 独立临时树和 target；0 模型请求 | 保留规范化日志并删除临时树；失败则停止 U2 | planned |
| V2 | 上游 substrate 能以很薄的 identity/home overlay 支撑 Whale CLI | 是否替换生产 vendor | 一次性临时候选树只应用品牌、二进制身份、`WHALE_HOME`、auth 隔离 patch | Enough：CLI build、version、home/auth 隔离测试通过；Not proven：需要 DeepSeek/TaskSpace stub | 不提交第二份 vendor；0 模型请求 | 删除临时树；失败则回到 seam 识别 | planned |

## 5. 可执行工作单元

### Phase A：候选方向验证

进入条件：工作树 clean；生产 vendor 未变化；0.146 身份证据可读取。适用决策：D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U1 | 纠正候选资格判断 | compatibility | 临时 0.146 tree；candidate manifest/report | 官方 build/test entrypoints | 执行 V1；只修 qualification runner 的环境隔离和命令解析，不改候选源码 | 可区分 harness、environment、upstream failure | 避免因假 blocker 启动大迁移或错误放弃候选 | Complexity：只删改现有 runner 窄逻辑；Reach/Cost：日志、manifest、报告，无生产影响 | 官方入口逐项结果；runner 单测；vendor diff 为零；重复运行一致 | 失败即 no-go，保持现 vendor，提交证据后停止 | not-started |
| U2 | 验证最小 Whale overlay | compatibility | 一次性临时 candidate tree | brand、binary identity、`WHALE_HOME`、auth isolation | 执行 V2；提取最小 patch，不带 DeepSeek/TaskSpace/cache | 候选可构建身份与数据目录隔离正确的 Whale CLI | 生产替换前验证关键 seam | Complexity：不得新增框架/双 vendor；Reach/Cost：临时构建和少量测试 | CLI build、version、home/keyring/auth tests；patch 清单可审阅 | 任一能力需要业务 stub 即停止并删临时树 | not-started |

退出条件：U1、U2 均 verified；否则停在当前 vendor。结束时审计本阶段 Product Decision Delta。

### Phase B：上游 substrate 落地

进入条件：Phase A verified，用户已看到资格结论。适用决策：D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U3 | 替换 vendor substrate | dependency/source | `third_party/codex-cli/`、`UPSTREAM.md` | 固定候选源码与 workspace metadata | 用已验证候选替换 vendor，只重放 U2 patch；锁文件同单元生成 | 生产 vendor 对齐上游，Whale 数据仍隔离 | 建立可维护 substrate，减少逐补丁漂移 | Complexity：大规模机械变化、无新业务抽象；Reach/Cost：全 workspace 构建和审查成本高 | SHA/tree/license；U2 tests；官方资格命令；overlay 仅含声明 patch | 独立提交整体 revert；发现未声明业务 patch 即停止 | not-started |

退出条件：U3 verified。不得在本阶段顺手修 DeepSeek/TaskSpace；其编译缺口只作为后续输入。

### Phase C：DeepSeek 闭环

进入条件：U3 verified。适用决策：D1、D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U4 | 恢复 provider 身份与鉴权 | provider | `model-provider-info` 及配置调用点 | DeepSeek provider identity、base URL、API key auth | 按新上游 provider 接口重放现有身份和鉴权逻辑，不引入模型目录与请求 payload | CLI 能识别并配置 DeepSeek provider | 先恢复最小接入点，隔离目录和 wire 问题 | Complexity：一个局部 provider 分支；Reach/Cost：配置解析和 auth tests | provider/config/auth 定向测试；0 真实请求 | 独立 revert；模型能力留给 U5 | not-started |
| U5 | 恢复模型目录与可见性 | catalog/defaults | `models-manager` 及 model selector | Flash/Pro catalog entries、default/visibility | 迁移模型元数据，保持 Flash 默认可见、Pro 隐藏；同单元更新目录生成物和快照 | 模型选择行为符合 D1 | 单独审计用户可见默认值 | Complexity：目录数据和现有 selector 条件；Reach/Cost：生成物和 TUI 快照 | model manager tests；默认/可见性快照；D1 检查 | 独立 revert；不得顺手实现 wire | not-started |
| U6 | 恢复 Responses 请求与流式事件 | provider-wire | `codex-api`、`core` 的 Responses/SSE seam | request mapping、reasoning stream、tool-call assembly | 按上游类型适配现有请求和事件组装，不触及 TaskSpace、用量、压缩或缓存 policy | reasoning 与 streamed tool calls 保持现行为 | 恢复主请求链并把后续状态行为隔离 | Complexity：Responses/SSE 局部分支；Reach/Cost：API/core fixtures | 无网络 request contracts、SSE reasoning/tool-call fixtures | 出现 TaskSpace 数据需求即停止并留给 U12/U13 | not-started |
| U7 | 恢复用量与请求预算 | accounting | `core/src/client.rs` 现有 accounting seam | provider usage、hard request limit、terminal reconciliation | 将现有计数与限额接到上游响应/事件类型 | 请求用量和终止对账保持一致 | 防止主线融合丢失成本与请求保护 | Complexity：局部计数状态；Reach/Cost：client/session accounting tests | usage fixtures、hard-limit、terminal reconciliation tests | 独立 revert；不修改 compaction threshold | not-started |
| U8 | 恢复 DeepSeek compaction | context-lifecycle | `core/src/compact*.rs` 及现有调用点 | Flash compact request、1M/755K threshold、保留状态 | 按新上游 context API 迁移现有 compaction 合同，不引入 TaskSpace projection 改动 | 长上下文收缩保持现有阈值和状态保留 | 将上下文生命周期风险从 wire/cache 中隔离 | Complexity：复用现 compaction 分支；Reach/Cost：context/session tests | compact request、threshold、retention、zero-short-job tests | 若必须修改 TaskSpace retention，停下留给 U13 | not-started |
| U9 | 恢复缓存与 final-wire 证据 | cache/observability | `provider_wire_*`、cache contract/gate | Standard 最终 payload 与敏感面 | 迁移 free final-wire/trace，运行 index gate；不建新缓存框架 | 可在零模型请求下检测 payload 漂移 | 付费回归前提供确定性保护 | Complexity：复用现合同、原则上无新 runtime state；Reach/Cost：cache-sensitive，可能预算阻塞 | contract tests；cache index gate；真实回归须另获批 | gate 阻断则回退/停在 U9，不用 `--no-verify` | not-started |

退出条件：U4–U9 verified；D1 语义 covered；没有 TaskSpace 产品语义变化。

### Phase D：TaskSpace 闭环

进入条件：Phase C verified。适用决策：D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U10 | 恢复 TaskSpace 领域模型 | domain | `core/src/action_map/**`、独立 protocol/tools 类型 | canonical map、DAG invariant、event types、tool schema | 重放不依赖存储/session/TUI 的领域模块，按新 workspace 类型做编译适配 | TaskSpace 业务模型可独立构建和验证 | 先固定业务含义，避免宿主接口反向塑形 | Complexity：保留现模块、不加新抽象；Reach/Cost：domain/protocol unit tests | invariant、serialization、tool schema tests | 若需改变状态语义，标 conflict 并请示 | not-started |
| U11 | 恢复持久化与 replay | data | state runtime、SQL migration、TaskSpace store | event store、CAS、migration、replay | 将 U10 类型接回现有 store；migration/schema 同单元生成 | canonical state 可持久化、迁移和重放 | 单独验证任务状态唯一权威 | Complexity：不建第二 store；Reach/Cost：SQL/schema/migration tests | CAS/store、migration upgrade、replay determinism tests | 无法保持单一权威则标 conflict 并请示 | not-started |
| U12 | 接回 tool identity/sequence | tool-runtime | core/tools registry、handler、sequence seam | control tool、call identity、preflight/terminal carrier | 在上游 seam 注册现有 handler，适配 call identity/response index | TaskSpace 控制调用恢复 | 从 session 生命周期中隔离工具冲突 | Complexity：窄 adapter/hook；Reach/Cost：registry、parallel calls、错误传播 | schema/handler、sequence、parallel、terminal tests | 不改 session/store 语义；所需内容留给 U13 | not-started |
| U13 | 接回 session/resume/fork | lifecycle | core/session、rollout reconstruction seam | projection、resume/fork/replay integration | 通过上游 session seam 投影 U11 状态，适配 resume/fork/compaction | 跨轮次行为可恢复和回放 | 完成核心生命周期并保持单一权威 | Complexity：session adapter；Reach/Cost：history、compaction、agent identity | resume/fork/replay/compaction/terminal tests；权威断言 | 若需并列状态权威，标 conflict 并请示 | not-started |
| U14 | 恢复 app-server 协议 | API | protocol、app-server TaskSpace adapter | RPC、JSON/TS schema、compatibility surface | 将 U10–U13 暴露到现有 RPC；schema/TS 与源同单元生成 | 外部客户端可通过版本化协议访问 TaskSpace | 单独验证协议兼容，不混 TUI 路由 | Complexity：现有 RPC adapter；Reach/Cost：schema generation 和 app-server tests | protocol/app-server tests；generation clean；兼容 fixtures | 独立 revert；不得引入 TUI fallback | not-started |
| U15 | 恢复 TUI 路由与 viewer | client | TUI TaskSpace route/viewer | route state、Action Map viewer、快照 | 按新 TUI seam 接入已验证 RPC/core 行为；快照同单元更新 | TaskSpace 在终端中保持现有可见交互 | 完成用户可见闭环并隔离 W9 | Complexity：现有 TUI adapter；Reach/Cost：route tests 和快照 | route/viewer tests；snapshot review；W9 按已知基线记录 | 不得为消除 W9 改业务路由规则 | not-started |

退出条件：U10–U15 verified；状态权威无冲突；W9 未新增回归且继续明确延期。

### Phase E：发布收口

进入条件：Phase D verified。适用决策：D1、D2。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| U16 | 完成发布闭环 | release | workspace metadata、`UPSTREAM.md`、migration report、CI/release config | 来源、剩余生成物、支持矩阵 | 刷新机械生成物和来源记录，跑 Linux 全量无模型回归，明确 Windows/W9 deferred | 形成可追溯新 vendor 和真实通过/延期清单 | 防止局部通过误报为整体完成 | Complexity：无新生产抽象；Reach/Cost：全 workspace build/CI | fmt/check/test、CLI smoke、schema/lock clean、cache gate、provenance、Git clean | 跨产品失败回到归属 U 单元，不在收口阶段扩张 | not-started |

退出条件：U16 verified；所有修改已提交并 push；工作树 clean；延期项未被表述为通过。

## 6. Product Decision Delta

每个阶段只追加一行，不回写 `decisions.md`。

| Phase | Decision Surface | Implemented / Observed Semantics | Authority Coverage | Classification | Required Action |
| --- | --- | --- | --- | --- | --- |
| 历史第一批 | 无产品语义 | 独立安全/通用 backport | none | engineering-only | 已收口 |
| 历史第二批 | 模型默认值与 Responses | Flash 默认、Pro 隐藏；Responses 按能力处理 | D1、D2 | covered | 保留为现行为证据，不作未来授权 |
| 历史第三批 | 无生产语义 | 仅候选、差异和 replay 证据；vendor 未变 | none | engineering-only | 自动语义分类已降级 |
| Phase A | 待执行 | 待记录 | D2 | 待分类 | U1/U2 后审计 |
| Phase B | 待执行 | 待记录 | D2 | 待分类 | U3 后审计 |
| Phase C | 待执行 | 待记录 | D1、D2 | 待分类 | U4–U9 后审计 |
| Phase D | 待执行 | 待记录 | D2 | 待分类 | U10–U15 后审计 |
| Phase E | 待执行 | 待记录 | D1、D2 | 待分类 | U16 后审计 |

## 7. Pending Product Decisions

当前没有必须在 Phase A 前决定的新产品行为。以下情况出现时必须停下请示：

- upstream AgentGraph/WorldState 与 TaskSpace Event Store 无法保持单一任务状态权威；
- 0.146 无法资格通过，需要改选不同上游版本；
- DeepSeek 官方 Pro Responses 支持状态变化，可能触发 D1 恢复条件；
- 新上游能力会改变默认权限、持久化、模型可见性或用户控制方式。

## 8. 执行与提交边界

严格按 `U1 -> U2 -> U3 -> U4 -> U5 -> U6 -> U7 -> U8 -> U9 -> U10 -> U11 -> U12 -> U13 -> U14 -> U15 -> U16` 推进。顺序只在新证据证明依赖错误时修订，不能用并行大合并绕过门禁。

- 每个 U 单元至少一个独立、可理解、已基本验证的 commit，并立即 push。
- 单元内出现两个可独立回滚的行为主题时继续拆 commit；不得把 vendor 机械替换与产品 overlay 混为一个提交。
- 本计划不授权新分支、真实模型请求、跨工作空间操作或超过仓库规模门禁的实现。
- 每阶段完成后先更新状态、证据链接和 Product Decision Delta，再进入下一阶段。

## 9. 计划验收

- [x] 只有一个产品决策权威源，且只含用户直接确认的 active 决策。
- [x] 历史批次与自动账本已从执行权威降级为证据。
- [x] 候选资格在大投入前先做最低成本复核。
- [x] 每个工作单元只有一个主要目标、变更轴和回滚边界。
- [x] DeepSeek provider/catalog、Responses wire、cache 分开验证。
- [x] TaskSpace domain/data、tool、session、client surface 分开验证。
- [x] 生成物与对应源变化同单元处理。
- [x] W9/Windows 延期不阻塞 Linux 主线融合，也不被误报为通过。
- [x] 不新增运行时框架、双 vendor、双状态权威或同步专用业务逻辑。
