# WhaleCode v0.0.6 多 Provider 工程实施计划

- Status: phase-3-in-progress
- Product Authority: `../../../../prd/2026-08-23-v0.0.6-multi-provider.md#confirmed-product-decisions`
- Applicable Decisions: PD1, PD2, PD3, PD4, PD5, PD6, PD7, PD8, PD9, PD10, PD11, PD12, PD13, PD14, PD15, PD16
- Current-State Evidence: `./current-state-inventory.md`
- Scope: Codex-derived Rust core、app-server protocol/server、认证、模型目录、TUI、rollout/replay、测试与发布文档

## Execution Contract

- Product Authority 中的 active 决策是本计划唯一用户权威；修改、重释或替换这些决策必须获得用户明确批准，Agent 不得自批。
- 已验证的代码、测试和运行证据可以修订本计划，但不得静默改写 Product Authority。
- 新的用户可见重大选择必须延期、标记为 provisional，或交由用户确认；未解决的 material `provisional` / `conflict` 会阻断依赖阶段。
- 每个物质阶段结束后，只审计该阶段引入或改变的 Product Decision Delta。
- 每个物质阶段开始前，必须用已完成实现和证据 rebase 全部剩余计划；gate 为 `pending` 或 `blocked-on-plan-approval` 时不得开始。
- 设计方向、范围、模块/API/数据边界、工作单元、顺序、验证、回滚、收益、成本或风险发生物质变化时，必须记录 Plan Delta 并获得用户明确批准，之后才能应用修订并继续依赖实现。
- 非物质性的局部实现细节可按代码事实调整，但应保持工作单元目标、外部行为、验证与安全停止边界不变。

## 1. Current And Expected Behavior

### Current

- OpenAI ChatGPT 与 API key 都可登录，但 `AuthDotJson`/`AuthManager` 只表达一个当前认证；API key 登录会覆盖 token。
- DeepSeek Provider 已存在，但只从 `DEEPSEEK_API_KEY` 环境变量取值；当前 DeepSeek Key UI 实际写入 OpenAI 字段。
- `/model` 只更新 model/effort，模型目录被过滤为 DeepSeek，缓存不含 Provider identity。
- `SessionConfiguration.provider` 在 session 初始化时固定；thread settings、turn context 和 replay 没有完整 Provider route。
- prompt、tools、commands、compaction、history projection 各自具备部分 capability seam，但没有原子 Provider transition。

### Expected

- 三条稳定访问路由：`openai/chatgpt`、`openai/api-key`、`deepseek/api-key`；路由只含非敏感 identity，不含凭据。
- `/provider` 和跨组 `/model` 选择生成同一种 session-scoped 原子 transition；active turn 中显示 pending，下一 turn 才采用新 route snapshot。
- OpenAI 两套凭据与 DeepSeek Key 隔离、安全共存、独立登录登出；每次请求明确绑定访问路由。
- provider、model、prompt、tools、commands、history projection 和 compaction policy 在同一个 prepared transition 中通过预检后一次提交。
- canonical rollout 不因切换被改写；目标 Provider 只收到兼容的 wire projection；resume/fork/rollback/subagent 能恢复正确路由。

## 2. Minimum-Sufficient Design

### 2.1 Stable Route Identity

在 `codex-protocol` 定义可序列化的 `ProviderRoute { model_provider_id, access_method }`，其中 `access_method` 首批只需 `Chatgpt` 与 `ApiKey`。模型选择由 `{ route, model, effort }` 表达。任何 rollout、状态、日志和 telemetry 只能记录该非敏感 route，不得记录 Key/token。

同一 `openai` wire Provider 可对应两个 route；因此缓存、最近模型和 UI 分组使用 route identity，工具/wire capability 仍使用实际 `ModelProviderInfo`/`ModelProvider`。

### 2.2 Credential Inventory And Route-Bound Auth

保留 Codex 原生 OAuth/device/API-key 登录流程和 token 刷新实现，扩展现有认证存储为版本化 credential inventory：ChatGPT token、OpenAI API key、DeepSeek API key 可共存。旧 `auth.json` 必须向后兼容读取并一次性无损投影到新结构；写入继续使用现有 file/keyring/auto 后端。

`AuthManager` 不再要求模型请求依赖一个进程级 active auth，而是提供 route-bound auth view。ChatGPT refresh 状态仍集中管理；API key view 只暴露目标 route 的 secret。登录/登出 API 必须显式携带 route，且只改变该槽。

### 2.3 Prepared Provider Transition

扩展 `thread/settings/update` 和 core `ThreadSettingsOverrides`，以一个可选 route+model selection 表达切换。handler 在持有旧 session authority 时完成凭据、模型 metadata、provider client、base instructions、history compatibility、context window/compaction 的预检，产出不含 secret 的 `PreparedProviderTransition`。

只有完整 prepared value 才能替换 `SessionConfiguration` 的 route、provider、model/prompt policy；失败时丢弃 prepared value。已捕获的 `TurnContext` 永不变更。active turn 期间 transition 排在其完成之后、下一条用户输入之前提交；TUI 的 pending 状态由请求发起到 `ThreadSettingsApplied`/失败响应闭环驱动。

### 2.4 Dynamic Surfaces Without A Mega-Registry

不新增通用插件框架。复用现有模型 metadata、`ModelProvider` capability、base-instruction resolver、tool spec builder 和 compact router，只补四个窄合同：route-scoped catalog、command availability、wire history projection、prepared transition。这样避免把当前三条路由扩张成无需求的 Provider framework。

### 2.5 Durable Context And Compatibility

扩展 `ThreadSettingsSnapshot`、`ThreadSettingsAppliedEvent`、`TurnContextItem` 与 `PreviousTurnSettings` 记录 route；新增字段保持 serde 默认以读取旧 rollout。replay 从有效事件序列恢复最后 route，并顺便派生 session 内每条 route 的最近成功模型，不新增独立全局默认状态。

历史在请求边界生成目标 route 的临时投影：先移除目标不支持的 provider-internal encrypted 字段和 hosted item/call-output 对，再复用现有 call/output、孤儿输出和媒体规范化。canonical history、compact checkpoint 和 rollout 原文保持不变。

跨 route 仅在 comp hash 或窗口约束要求时，于 transition commit 前使用上一 turn 的 route-bound provider 完成 pre-turn compact；失败即保持旧 route。commit 后的自动/手动 compact 使用新 route capability。

## 3. Pending Product Decisions

无。若实现证据暴露新的用户可见重大选择，相关阶段必须标记 `blocked` 并回到用户确认，不能以工程默认值补齐产品权威。

## 4. Pre-Investment Validation

| ID | Critical Assumption | Decision Unlocked | Cheapest Credible Method | Enough Evidence / Not Proven | Budget / Isolation | Stop / Cleanup | Status |
|---|---|---|---|---|---|---|---|
| V1 | 现有 file/keyring/auto 存储和 ChatGPT refresh 可被包装成多槽 inventory，而无需复制 OAuth/token-refresh 状态机 | W2–W4 的认证边界 | 为旧/新 `AuthDotJson` fixture、keyring mock 和两个 route-bound auth view 写离线 contract test；仅做最小类型 spike | 已证明现有 flat record 可同时承载 tokens/API key、keyring 可无损 round-trip、refresh 保留 API key；未证明真实凭据有效 | 无网络、无真实 Key、无模型费用 | 保留回归断言；按 D1 在 Phase 1 前审批简化后的存储边界 | direction-supported |
| V2 | 现有 submission FIFO 能保证 active turn 后、下一用户输入前提交 provider settings，且旧 `TurnContext` 不被突变 | W8、W13 的切换时序 | core mock provider 测试：active stream/tool continuation 期间提交 transition，再排队用户输入并记录每次 provider snapshot | 已证明 active turn settings 更新只影响下一 turn；Provider prepared value 的完整失败回滚仍由 W8 验证 | 本地 mock HTTP/内存 rollout，无真实模型 | 保留现有 FIFO seam；W8 若发现 Provider 特有差异再记录 Plan Delta | direction-supported |
| V3 | OpenAI 专属 reasoning/hosted history 可在不改 canonical history 的前提下投影为 DeepSeek 可接受输入 | W12 的 history projector | 用现有 `ResponseItemEnvelope` fixture 生成目标 route request，验证加密字段、hosted items、媒体和 call/output 配对 | 已证明请求格式化操作 clone 且不回写 canonical input，非 OpenAI request 可清除现有私有 metadata；完整 hosted/reasoning 矩阵留给 W12 | 离线 fixture/mock server；禁止真实 Whale run | 保留 clone-at-wire-boundary 方向；W12 仅扩展临时投影 | direction-supported |

## 5. Work Units

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| W1 | 固定 route identity | 协议/类型 | `protocol/src/provider_route.rs`、app-server v2 thread/account schemas | `ProviderRoute`、route+model selection DTO | 新增非敏感、向后兼容的 route 类型并生成 schema fixture | 三条访问路由在 core/app-server/TUI 使用同一 identity | 消除 provider ID 与 auth mode 混淆 | Complexity：新增 1 个值类型和 schema 字段；Reach：协议消费者与 fixture | serde 测试已通过；route 被 W7 API 引用后生成并审核 app-server schema fixture | 类型已实现且测试；API/schema 集成安全停在 W7 前 | implemented |
| W2 | 建立多槽凭据存储 | 安全/数据 | `login/src/auth/storage.rs`、`manager.rs`、storage tests | flat `AuthDotJson` credential slots | 按 D1 兼容扩展 `DEEPSEEK_API_KEY`，OpenAI/DeepSeek 登录执行读取—字段级合并—原子保存 | 三类凭据可共存且旧用户不丢登录 | 满足 PD9/PD15 并修复 DeepSeek 错位 | Complexity：新增 optional 字段；Reach：file/keyring/ephemeral、敏感数据生命周期 | 197 个 `codex-login` 测试全过，含旧 fixture、keyring 三槽 round-trip、refresh 保留 | optional 字段可被旧版本忽略；写入失败保持旧文件原子不变 | implemented |
| W3 | 提供 route-bound auth view | 认证运行时 | `login/src/auth/manager.rs`、`model-provider` client construction | `AuthManager::auth_for_route` | 从共享存储按 `ProviderRoute` 选择 ChatGPT/OpenAI API/DeepSeek API，不改变 legacy cached auth | 并发 session 可分别解析三条认证路径 | 防止切换一个 session 污染其他 session | Complexity：增加 route 选择分支；Reach：后续 provider request auth、refresh、401 路径 | 三路 route selection 与 key 隔离测试通过；client construction 接线留给 transition | 保留现有 legacy `auth()`，未迁移调用方行为不变 | auth-view-implemented |
| W4 | 精确登录登出与状态 | API/安全 | app-server protocol v2 account、`account_processor.rs`、TUI onboarding | login params、account status、logout target | 新增 DeepSeek 独立 login、三槽脱敏状态读取与 route 精确 logout；修正 TUI DeepSeek onboarding 的环境变量和登录类型 | 三类凭据可独立录入、检测和清除，legacy logout-all 保持兼容 | 闭合凭据生命周期并避免 Key 错槽 | Complexity：字段级 merge/clear；Reach：CLI/TUI/app-server 客户端 | protocol 292/292、account 目标路径与 legacy login/logout、TUI onboarding 15/15、login 197/197 通过 | legacy logout 保留；route 操作失败不改其他槽 | implemented |
| W5 | 隔离模型缓存 | 缓存/数据 | `models-manager/src/manager.rs`、cache types/tests | cache eligibility/path/key | cache entry 写入 optional route；route manager 使用独立安全文件名并严格匹配 route，旧无 identity cache 对 route manager 为 miss | 不同 Provider/访问方式不串模型目录 | 消除错误模型与 ETag 复用 | Complexity：cache schema/key 增加；Reach：启动/刷新/磁盘缓存，触发缓存敏感门禁 | `codex-models-manager` 54/54 通过，含 cross-route miss/refetch/store；三路 runtime manager 使用独立 cache；cache regression gate | legacy manager 继续读取旧 cache；新 route cache 可失效重建 | implemented |
| W6 | 生成三组模型目录 | 模型目录 | `models-manager`、bundled presets、app-server model listing、TUI `model_catalog.rs` | route-scoped catalog result | 保留 legacy DeepSeek 视图，新增按 route 获取/合并模型并携带 availability/reason | app-server 总能返回三组，缺凭据项显示不可用而非消失 | 满足 PD14/PD16，且路由明确 | Complexity：catalog DTO/三路刷新；Reach：启动延迟、缓存、picker、模型默认值 | route 分流、三组聚合、缺凭据可见、默认模型、Bedrock provider 目录隔离与 TUI 分组测试通过 | 单路刷新失败只标记该组不可用，不污染当前 route | implemented |
| W7 | 扩展原子 settings 协议 | 协议/控制面 | app-server `thread.rs`、`thread_processor.rs`、core protocol | `ThreadSettingsUpdateParams`、`ThreadSettingsOverrides` | 让 route+model 作为一个 selection 进入现有 settings submission | `/provider` 与跨组 `/model` 共享同一 core 操作 | 避免先切 provider 再补 model 的半状态 | Complexity：协议字段/映射分支；Reach：所有 settings clients/schema | request validation、legacy model-only、invalid combination tests | optional 新字段保持旧客户端兼容 | implemented |
| W8 | 构建并原子提交 transition | session authority | `core/src/session/session.rs`、`handlers.rs`、`turn_context.rs` | `PreparedProviderTransition`、`SessionConfiguration::apply` | 在 apply 前解析 route auth/provider/model/prompt/compatibility，成功后一次替换 session snapshot | 当前 turn 保持旧 route，下一 turn 使用完整新 snapshot | 实现根能力并保证失败回滚 | Complexity：新增 prepared state 和 apply 分支；Reach：turn creation、settings FIFO、metrics | V2；成功/失败/active turn/queued input/retry/tool loop tests | prepared value 未提交可直接丢弃；commit 前不改 session | implemented-runtime-foundation |
| W9 | 原子切换 prompt 与 runtime capability | prompt/tools | session init/base instruction resolver、world state、`tools/spec_plan.rs` | transition prompt snapshot、model-switch fragment、tool plan | 复用初始化优先级重算目标 base instructions，并从新 provider/model 重建工具；写入 provider-switch developer context | 新 turn 的 prompt/tools 与 route 同步，旧 turn 不受影响 | 避免旧提示词和错误工具暴露 | Complexity：resolver 参数化、world-state diff；Reach：缓存前缀、每步 tool spec、system/developer context | prompt precedence、OpenAI↔DeepSeek、tool capability snapshots | 失败不提交 transition；可退回旧 resolver 路径 | implemented-runtime-foundation |
| W10 | 持久化 route 与最近模型 | rollout/replay | protocol snapshots/items、session rollout reconstruction | `ThreadSettingsSnapshot`、`TurnContextItem`、`PreviousTurnSettings` | 增加 optional route，持久化 settings applied/turn context，并从有效事件派生 route 最近模型 | resume 后恢复最后成功 route，切回时恢复该 route 最近模型 | 支持连续性、审计与兼容旧 rollout | Complexity：schema字段/replay状态；Reach：resume/fork/rollback/truncation/subagent | old fixture、multi-transition replay、rollback/fork/recent-model tests | optional/default 兼容；旧 rollout fallback 到 SessionMeta provider | implemented-resume-and-recent-model |
| W11 | 切换压缩策略 | context/compaction | `core/src/session/turn.rs`、`tasks/compact.rs`、`compact.rs` | provider-aware previous settings、pre-transition compact | 用 previous route provider 判断 comp hash/window 并在 commit 前按需压缩，commit 后绑定新 compact capability | 只在必要时压缩且不会用错 Provider | 保留上下文并避免无条件成本 | Complexity：previous route 与失败分支；Reach：manual/auto/remote/local compact、checkpoint | comp hash/downshift/remote↔local/credential missing/resume tests | compact 失败保持旧 route；checkpoint 写入失败不提交 | implemented |
| W12 | 生成目标 Provider 历史投影 | wire compatibility | `core/src/context_manager`、`client.rs` | `project_history_for_provider` | 在 request serialization 前过滤 encrypted/provider-hosted/unsupported item 并运行既有 normalization | canonical 历史不变，目标 wire 输入兼容 | 支持同 thread 跨 Provider 连续对话 | Complexity：兼容矩阵/投影分支；Reach：所有请求历史、token 估算、tool pairing | V3；OpenAI→DeepSeek→OpenAI round-trip、pairing、media、aborted call tests | projector 只生成临时副本；异常时拒绝请求而不改历史 | implemented |
| W13 | 提供 `/provider` 交互 | TUI | `slash_command.rs`、slash dispatch、chatwidget popup/app events | Provider selection popup、pending state | 新增命令并展示当前/认证/默认或最近模型/可用原因，提交 route transition | 用户可在 active turn 选择并看到下一 turn 生效 | 满足主要入口与可恢复反馈 | Complexity：popup/pending/error states；Reach：slash dispatch、app-server events、onboarding | route-only submission、不可用原因、active-turn 提示、OAuth/API Key 恢复与原选择重试已覆盖 | 关闭 popup或失败只清 pending UI，不改变 core route | implemented |
| W14 | 改造分组 `/model` | TUI | `chatwidget/model_popups.rs`、`model_catalog.rs`、selection view | grouped routed model items | 按三组渲染全部模型；选择项提交 route+model，不执行跨 route 全局 config 持久化 | 一步完成跨 Provider 模型切换且计费路径明确 | 减少操作并避免无效全局 model/provider 组合 | Complexity：分组/availability/actions；Reach：reasoning popup、current/default 标记、旧 persist 行为 | 分组、不可用原因、route-aware reasoning 与原子 selection 定向测试通过 | 同 route 旧 `/model` 行为保持；跨 route 失败保留当前选择 | implemented |
| W15 | 呈现 Provider-aware 命令能力 | TUI/runtime capability | `bottom_pane/slash_commands.rs`、slash dispatch | existing command flags + disabled reason | 只对现有事实证明受限的 `/usage` 保持可见并展示与 dispatch guard 相同的 ChatGPT 登录原因；不新增通用 capability 层 | 不支持命令可见、禁用且原因一致 | 提升可发现性并阻止错误执行 | Complexity：复用现有 flag；Reach：命令菜单、键入路径、文案测试 | popup disabled reason 与 bare/args 直接输入测试通过 | 无已证明 Provider 限制的命令维持现有行为；`/apps` 保留管理入口 | implemented |
| W16 | 补齐脱敏与诊断 | observability/security | auth/provider errors、thread events、telemetry/status | route-safe fields、typed transition errors | 统一记录 route/stage/verdict，不记录 secret；为预检、排队、提交、投影、压缩失败提供 typed error | 用户和维护者能定位失败且凭据不泄露 | 降低支持与安全风险 | Complexity：错误枚举/字段；Reach：日志、rollout、status、telemetry | TUI secret masking、登录/刷新错误映射与既有 runtime 脱敏测试通过；发布扫描留 W18 | 日志字段可独立回退；错误不得包含底层 secret body | implemented-release-scan-pending |
| W17 | 闭合生命周期语义 | 集成/恢复 | thread resume/fork/rollback、agent spawn、app-server thread tests | route restoration/inheritance | 让 resume/fork/rollback 恢复历史位置有效 route，subagent 继承创建时 route snapshot | 多 Provider session 在全部生命周期路径一致 | 防止只在主对话 happy path 生效 | Complexity：重建/继承分支；Reach：thread manager、agent control、rollout truncation | lifecycle matrix tests，含立即退出和 pending/aborted transition | 失败时拒绝恢复并给 typed error，不猜测凭据 | implemented |
| W18 | 完成回归与交付证据 | 验证/文档 | targeted suites、isolated runner、本主题文档 | test matrix、evidence summary、release status | 运行分层离线回归、schema/fmt/clippy/cache gate并更新证据 | 可证明 PD1–PD16 的实现覆盖和剩余限制 | 提供可审查交付基线 | Complexity：无生产抽象；Reach：构建时间、测试维护、文档 | 见第 8 节；不运行未授权真实模型 | 任一 gate 失败停止发布；源码提交可按原子 commit 回退 | blocked-by-repository-baseline |

## 6. Phases

### Phase 0：关键假设验证与合同定型

#### Pre-Phase Plan Rebase Gate

- Rebase scope: PRD、当前 auth/session/history 实现、V1–V3
- Material plan delta: none
- Plan delta record: not-required
- User approval: not-required
- Gate status: verified

- Entry: PD1–PD16 active；不需要真实凭据或网络。
- Work: V1、V2、V3、W1 的 route/schema 最小合同。
- Exit evidence: V1–V3 均为 `direction-supported`；`ProviderRoute` serde 测试 2/2、login refresh 测试 1/1、keyring coexistence 测试 1/1、active-turn next-turn 测试 1/1、wire-copy/非 OpenAI 清理测试 2/2 通过；全程无真实 Key、网络 Provider 或模型费用。
- Product Decision Delta: `engineering-only`。实现未改变双槽、turn 边界或 canonical history 产品语义；D1 只简化后续私有存储设计。
- Formatting note: `just fmt` 因本机缺少 `dotslash` 在 Bazel formatter 前置步骤失败；相关 Rust 文件已用 stable `rustfmt` 格式化，代码定向测试均通过。
- Next: Phase 0 已 verified；D1 已于 2026-08-23 获用户批准，Phase 1 可以执行。

### Phase 1：凭据与模型目录基础

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase 0 类型/spike/测试结果 + W2–W6、W16 剩余设计
- Material plan delta: material
- Plan delta record: D1
- User approval: approved-2026-08-23
- Gate status: ready

- Entry: Phase 0 verified。
- Work: W2–W6 中的认证/缓存/目录部分，W16 的认证脱敏。
- Exit evidence: 三槽共存、独立 login/logout、route-bound auth 并发隔离、三组 catalog 和 cache 隔离测试通过。
- Product Decision Delta: 审计 PD3–PD5、PD9、PD13、PD15、PD16。
- Cross-unit side effects: auth storage migration 与 models cache 都是磁盘敏感面；必须分别验证原子写和旧格式兼容。

#### 当前实施证据

- 已完成范围：W2–W6 与认证侧 W16。三路凭据、route-bound auth、精确 login/logout、脱敏状态、route-scoped cache、分组目录、runtime manager registry、app-server `groups` 响应和 DeepSeek onboarding 均已接线；TUI `/provider` 与分组 `/model` 仍按 W13/W14 留在 Phase 4。
- `just test -p codex-login`：197/197 通过；全部为本地 fixture/mock，没有真实 Provider 请求或模型费用。
- `just test -p codex-models-manager`：52/52 通过；另一 route 或旧无 route entry 不会命中新 route manager。
- cache regression index gate：通过；surface `926188e1093dd019d07d8a8231c97592e3c3e5224c810c6481ef5e93043f6526`，Standard 与 TaskSpace 免费 final-wire 均可比较且无变化；未运行真实模型，发布级 live baseline 仍保持阻断。
- W6 目录合同：`ProviderModelGroup` 携带非敏感 route、组名、稳定 availability reason 与模型集合；route 分流区分 ChatGPT-only/OpenAI API/DeepSeek 模型，缺少 API key 的组保持可见。`codex-models-manager` 54/54 与协议序列化测试通过。
- W6 cache regression index gate：通过；surface `5a0b6fee9bb35398da33b4598a07dd14543c5450b62ff043476c70bf29c0879d`，免费 final-wire 无变化；未运行真实模型。
- W6 runtime/app-server：`ThreadManager` 持有 `openai/chatgpt`、`openai/api-key`、`deepseek/api-key` 三个 route-bound manager，认证、模型 endpoint 与磁盘 cache 都按 route 解析；`model/list` 在保留旧 `data`/cursor 的同时返回真实 `groups`。`codex-model-provider` 76/76、`codex-models-manager` 54/54、app-server model-list 5/5 通过；Bedrock 权威目录显式隔离于 Whale legacy DeepSeek 过滤。
- W6 runtime cache regression index gate：通过；surface `311ebcae9d38fec7a3bae0d23a1ca279c00ffb3cc50c875241ca652b71ee24b8`，免费 final-wire 验证通过；未运行真实模型，发布级 live baseline 继续阻断。
- 兼容行为：损坏的旧 auth 仍可被原生登录修复；有效旧记录执行字段级合并；OpenAI API 登录继续清除互斥的 Bedrock 激活状态，但保留 ChatGPT 与 DeepSeek 槽。
- W4 API/TUI：app-server protocol 292/292、`codex-login` 197/197、TUI onboarding 15/15、account 聚合回归 52/52 通过；account 路径覆盖 DeepSeek login→三槽脱敏 read→DeepSeek-only logout、legacy OpenAI login/logout、Bedrock 回退与 Edu workspace plan。共享测试服务器补齐空 managed cloud-config bundle fixture，避免 workspace auth 回退到默认配置。
- W4 cache regression index gate：通过；surface 仍为 `311ebcae9d38fec7a3bae0d23a1ca279c00ffb3cc50c875241ca652b71ee24b8`，免费 final-wire 无变化；未运行真实模型。
- Phase 1 Exit：verified。三槽共存、独立 login/logout、route-bound auth 隔离、三组 catalog、cache 隔离与 secret-free status 均有离线证据；全程未发起真实 Provider/模型请求。

### Phase 2：Core 原子 Provider Transition

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase 1 已实现的三路 credential/catalog registry + 当前 settings FIFO、`SessionConfiguration`、`TurnContext`、compaction 实现 + W7–W11、W16
- Material plan delta: material（D2）
- Plan delta record: D2、D3
- User approval: user-approved-plan-direct: 2026-08-23（D2 + Phase 2 手写生产代码上限 1200 行；D3 + 上限调整为 1350 行）
- Gate status: ready

- Entry: route auth/catalog 可由 mock 稳定解析；V2 direction-supported。
- Work: W7、W8、W9、W10、W11 和 transition 诊断。
- Rebased execution slices:
  1. W7 + W10 contract：把 `{route, model, effort}` 作为单一 selection 贯穿 app-server/core settings，并先持久化 optional route；legacy model-only 继续沿当前 route。
  2. W8 runtime：在 session services 注入窄 `ProviderRuntimeRegistry`，每个 route 绑定 `ModelProviderInfo`、route-bound provider factory 与 Phase 1 models manager；prepare 阶段完成凭据/模型/provider 校验，commit 阶段只交换已构造 snapshot。
  3. W9 surfaces：基于 prepared provider/model 重新解析 base instructions，工具继续由新 `TurnContext` 的 provider/model capability 生成，不新增常驻 mega-registry。
  4. W11 compact：`PreviousTurnSettings` 增加 route；需要 pre-transition compact 时显式使用旧 turn snapshot，成功后才提交新 route。
- Estimated handwritten production code: 900–1350 行；本阶段经 D3 批准的上限为 1350 行，测试/schema/generated/docs 不计入。若达到上限仍不能闭合原子 transition，停止扩张并重新审批。
- Exit evidence: active turn/queued input/tool loop/compact 下旧 turn 不变、下一 turn 完整切换；失败无半状态；rollout 可恢复。
- Product Decision Delta: 审计 PD6–PD11；任何同 turn 热替换均为 `conflict`。
- Cross-unit side effects: session snapshot 扩大、base prompt 前缀变化、compact 请求归属改变；必须一并验证缓存与费用 route 字段。
- Current evidence (2026-08-23): W7 schema 与 core/app-server 映射完成；W8 prepare 会在 commit 前离线验证 route credential、目标模型、provider runtime 与 base instructions，失败不改 session；每个 turn 的 `ModelClientSession` 绑定其不可变 provider snapshot，WebSocket/sticky state 不跨 provider；legacy/custom provider 仅在 effective provider 与命名配置一致时绑定内建 route，避免错误猜测 DeepSeek；W10 optional route 已进入 settings/turn-context/rollout reconstruction；W11 previous-route compaction 已使用旧 route provider、catalog 与 client。默认线程栈下 previous-model compact 5/5、`codex-core` provider-binding 1/1、protocol route round-trip 1/1、app-server settings suite 9/9 通过；stack-overflow 诊断与修复证据记录于 `../../../../coe/2026-08-23-20-45-provider-compact-stack-overflow.md`。
- D3 transition compensation evidence (2026-08-23): provider/model 选择进入带 revision 的 pending 状态；下一 turn 先使用旧 route 执行必要压缩，成功后以 revision finalize，压缩失败则以同一 revision 恢复稳定 route/model。旧 turn 的失败无法撤销更新的选择；补偿只回滚 route/model/runtime/base prompt，不覆盖后续独立的 Plan/Default 模式、模式指令、reasoning summary、权限或环境设置。state 并发/补偿测试 2/2、compact 失败集成测试 1/1、previous-model compact 5/5 通过。
- Active-turn route evidence (2026-08-23): app-server 在旧 provider 请求仍处于 active 时接受跨 route settings update，通知立即呈现目标 OpenAI API route/model，而已发出的请求仍保持旧 `mock_provider`/`mock-model`；定向测试 1/1 通过。目标 route 的凭据预检与 route-bound client 请求由 W3/W8 既有测试独立覆盖，未为重复断言引入双协议 mock 基础设施。
- Phase 2 current-slice cache regression index gate：通过；surface `08a45ae670503ef84aa668bf3842fb5b0ec0e2a8567a23376618a0cb1c6bd9ba`，免费 final-wire 可比较且无回归；未运行真实模型，发布级 live baseline 继续阻断。
- Phase 2 Exit：verified。active-turn 跨 route 1/1、带 tool continuation 的 next-turn settings 1/1、turn-scoped provider client 绑定 1/1、pending revision/CAS 2/2、compact 失败补偿 1/1、previous-model compact 5/5、app-server settings 10/10、core/app-server check 与 staged cache gate 均通过。route 已进入协议、turn context 与 rollout；从有效 rollout 恢复 session route 和每条 route 最近成功模型需要同时处理 rollback/fork/resume，按原计划归入 Phase 3 的 W10/W17，避免先引入不可重放的内存 map。

### Phase 3：History Projection 与生命周期恢复

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase 2 durable route/compact 实现 + W12、W17
- Material plan delta: material（D4 proposal）
- Plan delta record: D4
- User approval: user-approved-plan-direct: 2026-08-23（“批准，继续”）
- Gate status: verified

- Entry: transition/replay 核心测试 verified；V3 direction-supported。
- Rebased execution slices:
  1. W10 + W17 lifecycle snapshot：让现有 reverse rollout reconstruction 在同一有效段/rollback 语义下产出最后成功 route 与 route→最近成功模型；resume/fork/rollback 只通过 runtime registry 重新绑定完整 provider/model/prompt snapshot，不新增全局默认或不可重放内存旁路。
  2. W12 wire projection：在 request 边界从 canonical history clone 生成目标 provider 投影，先闭合 encrypted/provider-hosted item 与 call/output 配对，再覆盖媒体；canonical rollout、checkpoint 和原 history hash 保持不变。
  3. Lifecycle consumers：subagent 继承创建时的 route snapshot；缺凭据或目录不兼容时返回带 route/stage 的脱敏错误，不猜测替代 provider。
- Estimated handwritten production code: 650–950 行；建议 Phase 3 上限 1000 行，测试/schema/generated/docs 不计入。达到上限仍不能闭合 replay + projection 时停止扩张并重新审批。
- Lifecycle slice evidence (2026-08-23): reverse rollout 只从成功、未被 rollback 的 user-turn segment 派生 active route 与 route→最近成功模型；冷 resume 在 `SessionConfigured` 前经 runtime registry 重绑 provider/models manager/model/base prompt，缺 route/credential/model 时返回含 route+stage 且不含 secret 的错误。成功 turn 更新 session 派生缓存；仅切 route 时优先最近成功模型、再回退目标 catalog 首选。rollback/recent-model 测试 1/1、OpenAI API↔DeepSeek 冷恢复与往返测试 1/1、`cargo check -p codex-core` 通过；全部使用本地伪凭据与离线 catalog，无真实模型请求。
- Wire projection evidence (2026-08-23): DeepSeek 路径按仓库现有原生 Responses contract 保留可读 reasoning，只在 request clone 上清除 OpenAI 私有 reasoning、function args、agent/tool output 密文；provider-hosted search/image、opaque compaction 和 unknown request controls 不进入非 OpenAI wire。私有-only tool output 与 call 成对移除，随后复用既有 call/output 和媒体 normalization；切回 OpenAI 时原 clone 保持完整。投影单元测试 6/6、既有 non-OpenAI request 集成测试 1/1、`cargo check -p codex-core` 通过，无真实模型请求。当前 Phase 3 手写生产代码净增约 381/1000 行。
- Fork/subagent evidence (2026-08-23): 普通 history fork 继续从所选历史切点恢复最后成功 route；full-history 子 Agent 则保留 spawning turn 已捕获的完整 route/model snapshot，避免父会话刚切换 provider 时被旧成功 turn 反向覆盖，并把该 snapshot 记入子 session 的 route 最近模型派生缓存。扩展冷恢复测试同时覆盖 DeepSeek→OpenAI spawning-turn 子 fork 与同一历史的普通 root fork，1/1 通过；`cargo check -p codex-core` 通过。当前 Phase 3 手写生产代码净增约 395/1000 行。
- Exit evidence: OpenAI→DeepSeek→OpenAI fixture round-trip，canonical history clone 不变；resume/rollback/recent-model/subagent fork route 正确，普通 fork 保留历史切点语义。Phase 3 verified。
- Product Decision Delta: 审计 PD6、PD7、PD8、PD10、PD11。
- Cross-unit side effects: wire token 数可能因投影变化；只记录差异，不把 mock 结果声称为真实 Provider 接受性。

### Phase 4：TUI 统一交互与命令能力

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase 1–3 最终 protocol/events/errors + W13–W15
- Material plan delta: material（D5 proposal）
- Plan delta record: D5
- User approval: user-approved-plan-direct: 2026-08-24（“批准，继续”）
- Gate status: ready

- Entry: app-server/core transition API integrated；三组 catalog 可用。
- Rebase facts (2026-08-23): app-server `model/list` 已返回带 route/availability/reason 的三组 `groups`，但 TUI bootstrap 只保留扁平 `data`；现有 `ModelCatalog` 与 model/reasoning popup 只携带 `ModelPreset`，无法原子表达跨 route 选择。`UpdateModel` 会先改本地 canonical model，`PersistModelSelection` 会写全局配置，与 PD7/PD8 的 session-only 和 notification-authoritative 语义冲突。三类 `account/login/start` 协议均已存在，但运行中 TUI 尚无可复用的 provider 登录入口。
- Rebased execution slices:
  1. Catalog contract：保留 `model/list.groups` 到 TUI route-aware catalog；legacy server 无 groups 时仅为当前 route 合成兼容组，不猜测其他 route。
  2. Routed selection：`/provider` 提交 route-only selection，由 core 选择 route 最近成功/默认模型；`/model` 的分组项与 reasoning 子弹窗始终携带 route+model，通过一次 settings update 提交。跨 route 不写全局配置，同 route 的既有 model-only 持久化行为保持兼容。
  3. Next-turn indication：不复制 core 的 pending/CAS 状态机。settings 请求成功且旧 turn 仍在运行时，仅记录一个短生命周期“下一轮生效”提示；canonical route/model 继续只由匹配 active thread 的 `ThreadSettingsUpdated` 更新，请求失败保持旧状态。
  4. Credential recovery：不整体搬用启动 onboarding 状态机；复用已有 ChatGPT/API Key/DeepSeek `account/login/start` 协议、浏览器 OAuth 流程和 secret 输入组件，登录成功后刷新分组再继续原选择，不新建凭据格式。
  5. Command availability：不新增通用 Provider capability 框架。只扩展现有 command flags/dispatch guard，对已有 auth/model 事实可证明受限的命令显示禁用原因；`/apps` 保留管理入口，工具暴露继续由 provider `namespace_tools` capability 决定。其他命令维持现有行为。
- Approved implementation split:
  - Phase 4A（上限 500 行）：route-aware catalog、`/provider`、分组 `/model`、单一 routed selection event、settings 原子提交与下一轮提示。
  - Phase 4B（上限 500 行）：三类凭据恢复、登录后 catalog 刷新、真实受限命令的可见禁用与直接输入 guard。
- Phase 4A evidence (2026-08-24): TUI bootstrap 保留 app-server route groups；`/provider` 提交 route-only selection，`/model` 按 provider 分组并让 reasoning 选择携带 route；两者共用一次 `ThreadSettingsUpdateParams` 原子提交。缺凭据组可见但禁用并展示原因；active turn 请求成功时提示下一轮生效，canonical state 仍只由 thread settings notification 更新。同 route 模型选择继续持久化旧默认，跨 route 不写全局配置。`provider_and_model_pickers_preserve_route_groups`、`route_only_update_is_not_dropped` 通过，`cargo check -p codex-tui` 与全测试编译通过；手写生产代码净增 314/500 行。Phase 4B 的登录恢复、catalog 刷新与命令禁用仍待实施。
- Phase 4B evidence (2026-08-24): 缺凭据 route 保持“Unavailable”状态但可选择进入恢复；OpenAI 订阅复用 `LoginAccountParams::Chatgpt` 浏览器 OAuth，OpenAI API 与 DeepSeek 分别复用 `ApiKey`/`DeepseekApiKey`，API Key 只在遮罩输入和请求内存中短暂存在。登录成功后重新执行 `model/list`、替换同一 `ModelCatalog` 并重试原 route/model/effort selection；失败不提交 settings。命令层只修正已证明受限的 `/usage`：popup 保持可见并显示与直接 dispatch 相同的 ChatGPT 登录原因，`/apps` 和其他命令不受影响。picker recovery、selection preservation、secret masking、popup disabled reason、bare/args guard 定向测试通过，`cargo check -p codex-tui` 与全测试编译通过；模型 popup 快照已与既有 vision 模型目录对齐并复验通过。staged cache regression gate 通过，指纹 `cc32098712b47403205ac354cadb696fe1bcb9952db1703010cc4b7a2d9dc909`。手写生产代码净增 419/500 行，无真实凭据或模型请求。隔离 TUI 全量回归为 3709/3755 通过、46 失败、6 跳过；失败横跨 feedback/guardian/status/pets 等既有模块，且可独立复现已存在的 `/ag` 前缀测试与 `Agent`/`Agents` 冲突，因此不将全量回归误报为通过，剩余基线分类留 W18。
- Work: W13、W14、W15 和 UI 侧 W16。Phase 4 手写生产代码总上限 1000 行，且每个切片上限 500 行；测试/schema/generated/docs 不计入。任一切片达到上限仍未闭环时停止扩张并重新审批。
- Exit evidence: `/provider`、分组 `/model`、pending/取消/失败/登录、不支持命令交互 snapshots 全通过。
- Product Decision Delta: 审计 PD2、PD3、PD8、PD12、PD14、PD16。
- Cross-unit side effects: 跨 route `/model` 不写入无对应 provider 的全局 model；同 route 既有 reasoning 选择流程保持兼容。

### Phase 5：集成回归与发布候选

#### Pre-Phase Plan Rebase Gate

- Rebase scope: 全部实现、schema/fixture、缓存指纹、回归结果、v0.0.6 文档
- Material plan delta: none
- Plan delta record: not-required
- User approval: not-required
- Gate status: ready

- Entry: Phase 1–4 功能切片 evidence verified；隔离 TUI 全量基线已知为 3709/3755 通过、46 失败、6 跳过。
- Rebase facts (2026-08-24): Phase 4 相关 model popup 陈旧快照已对齐并单测通过；剩余失败横跨 feedback、guardian、status、pets 等未属于 multi-provider 的模块。其中 `/ag` 失败可独立定位为既有 `Agent`/`Agents` 前缀选择冲突；ChatGPT rate-limit prefetch 失败来自测试使用 DeepSeek 默认 provider 却期待 `requires_openai_auth=true`。W18 不会为追求全绿扩张修复这些无关功能，也不会把失败误报为通过；它们若在最终矩阵中保持，发布状态仍为 blocked，并给出精确证据。
- Work: W18；修复仅限已批准设计内缺陷，物质范围变化先记录 Plan Delta。
- Exit evidence: 第 8 节必需门禁全部通过，PRD acceptance criteria 有逐项测试证据，工作区无本任务未提交修改。
- Product Decision Delta: 汇总各阶段审计，不用实现结果反向扩展 Product Authority。
- Cross-unit side effects: 完整 vendor 回归耗时较长；使用隔离 runner，禁止宿主共享临时目录造成误判。
- Current evidence (2026-08-24): 隔离命令 `python3 scripts/codex-upstream/run_isolated_tests.py -p codex-login -p codex-models-manager -p codex-protocol -p codex-core -p codex-app-server -p codex-tui` 执行 9284 项，8928 通过、356 失败，耗时 244.568s。分组为 login 197/197、models-manager 54/54、protocol 296/297、core 3485/3742、app-server 1187/1239、TUI 3709/3755。JUnit 本地证据为 `third_party/codex-cli/codex-rs/target/nextest/local/junit.xml`（build artifact，不提交）。
- Relevant failure triage (2026-08-24): multi-provider 新增定向测试均通过；额外抽查 `model_switch_to_smaller_model_updates_token_context_window` 发现旧测试在 Whale 默认 DeepSeek manager 上注入 OpenAI 远程模型，因 route 目录过滤而失败；`read_default_provider_capabilities` 在实际返回 DeepSeek `namespace_tools=false`/`image_generation=false` 时仍期待 OpenAI 的 `true/true`。两者都是测试前提未显式选择 OpenAI route，不是当前 transition 或 capability 实现的反证；修复整个仓库基线超出本主题批准范围。
- Static gates (2026-08-24): `cargo fmt --all -- --check` 通过。`just fmt-check` 因本机缺失 `dotslash` 且仓库已有 nightly `imports_granularity` 格式差异失败；`just clippy -p codex-tui` 在未触及的 `state/src/runtime/taskspace_action_settlements.rs` 被 `clippy::expect_used` 阻断。两项均如实保留为发布阻断，未修改无关源码绕过。
- Phase 5 status: blocked。功能实现已原子提交且定向门禁通过，但 Completion Definition 要求的绝对全量回归、`just fmt-check` 与 clippy 未通过，v0.0.6 multi-provider 不能标记为 release-ready。

## 7. Product Decision Delta Log

| Phase | Decision Surface | Implemented / Observed Semantics | Authority Coverage | Classification | Required Action |
|---|---|---|---|---|---|
| Phase 0 | route/auth/turn/history 可行性 | route identity 不含 secret；现有 flat auth record 支持 OpenAI 双槽存储/刷新；settings FIFO 保持 next-turn 边界；wire projection 基于副本 | PD3–PD6、PD8、PD11、PD15 | engineering-only | D1 仅修改后续工程设计；Phase 1 前请求计划审批 |
| Phase 1 | 凭据与模型分组 | 三条 route 的凭据可共存并精确读写/登出；状态仅暴露 configured 布尔值；目录和缓存按 route 隔离，缺凭据组保持可见；DeepSeek onboarding 不再写入 OpenAI 槽 | PD3–PD5、PD9、PD13、PD15、PD16 | conforming | 无新增产品决策；进入 Phase 2 rebase |
| Phase 2 | 原子切换、prompt/tools/compact/replay | session runtime registry、prepared snapshot、route-bound turn client、route rollout 字段、旧 route compact 与 revision/CAS 补偿已闭合；补偿只覆盖 provider/model 选择，不覆盖后续独立 settings | PD6–PD11 | conforming | D2、D3 已实施并通过门禁；Phase 2 verified |
| Phase 3 | 历史与生命周期 | resume/rollback/普通 fork 从有效 rollout 重绑历史位置 route/model/prompt；subagent fork 保留 spawning-turn snapshot；目标 provider request 使用可逆 history clone 投影 | PD6、PD7、PD10、PD11 | conforming | D4 已实施并通过门禁；Phase 3 verified，进入 Phase 4 rebase |
| Phase 4 | TUI 与命令可用性 | route-aware `/provider`、分组 `/model`、三类凭据恢复、catalog 刷新及真实受限命令原因已闭合 | PD2、PD3、PD4、PD5、PD8、PD12、PD14、PD16 | conforming | Phase 4 定向门禁通过；隔离 TUI 全量基线 46 个失败留 W18 分类，不影响本切片原子提交 |
| Phase 5 | 整体验收 | 受影响六 crate 隔离矩阵、fmt 与 clippy 已执行；功能定向证据通过，仓库基线门禁未通过 | PD1–PD16 | blocked | 不扩张修复无关基线；需单独批准基线修复范围后重跑 W18 |

## 8. Verification Strategy

### Per-Unit Fast Loop

- 格式：`cd third_party/codex-cli && just fmt-check`。
- 定向测试：在 `third_party/codex-cli` 使用 `just test -p <crate> <test-filter>`；优先 login、models-manager、protocol、core、app-server、tui 的最小相关 suite。
- 协议：`cd third_party/codex-cli && just write-app-server-schema`，审查生成 diff 后重跑 protocol/schema tests。
- 静态检查：受影响 crate 使用 `just clippy -p <crate>`，不为未触及 crate 扩大日常反馈环。

### Required Integration Matrix

- Routes: OpenAI 订阅 ↔ OpenAI API；OpenAI → DeepSeek → OpenAI；同 route model-only。
- Auth: 空/错 Key、取消、覆盖保护、独立 logout、ChatGPT refresh、并发 session 不串凭据。
- Timing: idle、active stream、tool continuation、retry、manual/auto compact、queued input、transition 后立即退出。
- History: encrypted reasoning、hosted web/image、function/custom/namespace tools、媒体、pending/aborted call、compact checkpoint。
- Lifecycle: resume、fork、rollback、clear、subagent inherit、legacy rollout。
- Catalog/UI: 三组、同名模型、缺凭据不可用原因、单组刷新失败、最近模型/default fallback、不支持命令。
- Security/observability: auth files/keyring fixtures、rollout、logs、telemetry、errors 均无 secret。

### Final Gates

1. `cd third_party/codex-cli && just fmt-check`。
2. 受影响 crate 的 `just clippy -p ...` 与定向 `just test -p ...`。
3. 从仓库根运行 `python3 scripts/codex-upstream/run_isolated_tests.py <nextest args>` 覆盖 login、models-manager、protocol、core、app-server、tui；不得用宿主代理或共享临时目录失败判断回归。
4. staged cache-sensitive 变更运行 `python3 scripts/cache-regression/check_cache_regression_gate.py --source index`；阻断时按仓库规则说明指纹和前缀风险并申请真实回归预算，禁止绕过。
5. `git diff --check`、schema fixture diff 审核、敏感字段扫描、PRD acceptance criteria 到测试证据映射。
6. 默认不发起真实模型请求。若 mock/fixture 无法证明 Provider 实际接受性，另行申请最多 3 sample 或专项预算并先登记全局 ledger；未授权不得运行。

## 9. Safe Delivery And Commit Boundaries

- 每个 W 单元或紧密耦合的最小闭环完成并通过相关测试后原子 commit/push；不得把凭据、cache fixture secret 或用户数据提交。
- 优先顺序：兼容 schema/读取 → 新行为 behind complete call path → caller migration → legacy adapter cleanup。任何中间 commit 必须可编译或明确限定为纯测试/文档合同。
- 不使用不可恢复删除；旧 auth/cache 数据迁移必须 copy-on-write/atomic replace，失败保留原文件。
- 不新增分支；除非用户另行批准。
- 若单阶段预计新增超过 500 行手写生产代码，或单个 Whale 自有文件将超过 500 行，阶段开始前拆分更小实现或向用户申请范围批准；vendor 上游原文件长度例外不等于新增代码预算例外。

## 10. Plan Delta History

| ID | Before Phase | Previous Plan | Current Fact | Proposed Change | Impact | User Approval | Status |
|---|---|---|---|---|---|---|---|
| D1 | Phase 1 | 新建版本化 credential inventory，并把旧 `AuthDotJson` 迁移到新结构 | 现有 `AuthDotJson` 已能同时保存 ChatGPT tokens 与 OpenAI API key；file/keyring round-trip 可保留两者；`persist_tokens` 原位刷新并保留 API key | 保留现有 flat OpenAI 字段作为双槽权威，仅新增 optional `DEEPSEEK_API_KEY` 槽；登录/登出改为字段级 merge/clear，route-bound auth 显式选槽；不新增 inventory 版本或嵌套迁移层 | 减少 schema/migration/兼容分支和批量迁移风险；W2/W3/W4 目标、产品行为和验证矩阵不变 | approved-2026-08-23 | accepted |
| D2 | Phase 2 | `PreparedProviderTransition` 直接在现有 session authority 内解析目标 provider/catalog | `Session` 只持有启动时的一套 `SharedModelProvider` 与 `SharedModelsManager`；Phase 1 的三路 manager registry 当前归 `ThreadManager`，而 runtime provider 的 auth 仍走 legacy active auth；仅扩展 settings 字段会产生 provider、manager 与 route 不一致 | 新增仅覆盖三条已确认 route 的窄 `ProviderRuntimeRegistry` 并注入 session services；新增 route-bound runtime provider factory；prepare 产出 provider + models manager + model metadata + prompt policy 的完整值，commit 不再执行 I/O | 模块边界扩大但产品行为不变；消除半切换和 legacy active-auth 串路风险；预计本阶段 900–1200 行手写生产代码，上限 1200 行 | user-approved-plan-direct: 2026-08-23 | accepted |
| D3 | Phase 2 | settings 提交后立即替换稳定 session snapshot，下一 turn 的必要压缩沿用目标 route | 压缩必须使用旧 route/model 的窗口、hash 与 compact client；且 settings 可在压缩期间被再次更新，普通回滚会覆盖较新的用户选择 | provider/model 选择先进入带 revision 的 pending transition；下一 turn 用旧 route 完成必要压缩后 CAS finalize，失败则只补偿同 revision 的 provider/model/runtime/base prompt | 新增窄 pending 状态与补偿分支；避免用错 Provider 压缩，也避免旧失败撤销较新选择；Phase 2 手写生产代码上限调整为 1350 行 | user-approved-plan-direct: 2026-08-23 | accepted |
| D4 | Phase 3 | 分开实现 route 最近模型、resume 恢复、rollback/fork 和历史投影 | route 已写入 turn/rollout，但 resume 只恢复 previous-turn metadata；独立内存 recent-model map 无法遵守 rollback/fork/replay；canonical history 也不能为某个 Provider 原地改写 | W10 与 W17 合并复用 reverse rollout 的有效段语义，派生最后成功 route 及 route→最近成功模型，再由 runtime registry 重绑完整 snapshot；W12 只在请求边界投影 canonical history clone；subagent 继承创建时 snapshot；错误保持 route/stage 脱敏信息 | 生命周期状态可重放且不新增全局旁路；历史切换可逆；Phase 3 按三个闭环切片实施，手写生产代码上限 1000 行 | user-approved-plan-direct: 2026-08-23（“批准，继续”） | accepted |
| D5 | Phase 4 | 直接在现有扁平 `ModelCatalog` 和 `UpdateModel`/`PersistModelSelection` 上增加 `/provider` 与分组样式 | TUI bootstrap 丢弃 app-server 已返回的 route groups；model/reasoning 事件无 route；本地模型会在 server 确认前改变并无条件写全局配置；运行中无 provider 登录入口 | 保留 route-aware groups；新增仅供 provider-aware picker 使用的 routed selection event；canonical state 继续由 `ThreadSettingsUpdated` 确认，active turn 只增加短生命周期下一轮提示而不复制 core pending 状态机；跨 route 只写 session；登录只复用现有协议/OAuth/secret 输入；命令沿用现有 flags/guard | 避免伪分组、半切换、全局污染，同时删除长期 pending 镜像和通用 capability 框架；拆为两个各不超过 500 行的闭环，Phase 4 总上限 1000 行 | user-approved-plan-direct: 2026-08-24（“批准，继续”） | accepted |

## 11. Completion Definition

- PD1–PD16 的每项 acceptance behavior 均有自动化测试或明确、可复现的离线证据。
- 三类凭据安全共存且独立注销；无 secret 出现在日志、rollout、telemetry、错误或 Git。
- `/provider` 与分组 `/model` 在 active turn 中只显示 pending，下一 turn 原子生效；失败保持旧 route。
- prompt、tools、commands、history projection 和 compaction 使用同一 route snapshot。
- resume/fork/rollback/subagent 与 legacy rollout 恢复行为通过。
- schema、fmt、clippy、targeted tests、isolated regression、cache regression gate 全部通过。
- 本计划的 Phase gates、Product Decision Delta、Plan Delta 与证据链接已更新；工作区无本任务遗留未提交修改。
