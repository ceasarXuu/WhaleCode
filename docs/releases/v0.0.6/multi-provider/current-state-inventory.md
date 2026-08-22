# 多 Provider 当前实现与差距清单

- 盘点日期：2026-08-23
- 盘点范围：Codex vendor 中的认证、Provider、模型目录、TUI slash command、session、提示词、工具、压缩、历史与恢复链路
- 结论性质：代码事实基线；设计建议与未确认产品选择单独标注

## 1. 总结论

不能把本主题实现成“新增 `/provider` 弹窗，然后改写 `model_provider` 配置”。当前系统的 Provider 是 session 启动时构造的运行时对象，而 `/model` 只修改模型与 reasoning effort。真正的中途切换需要一个可验证、可回滚、可持久化的原子 Provider transition。

现状可分为三类：

| 能力面 | 当前状态 | 对本主题的含义 |
|---|---|---|
| OpenAI 订阅与 API | 同一 `openai` Provider 下已有 ChatGPT OAuth 与 API Key 两种原生认证模式 | 可复用认证机制，但当前只维护一个 active OpenAI auth |
| DeepSeek Provider | 已内置 Responses Provider 与 capability；运行时从 `DEEPSEEK_API_KEY` 环境变量取 Key | 缺少与“用户填入并安全保存”一致的录入、持久化和读取闭环 |
| `/model` | 可在同一 Provider 内更新模型，下一 turn 生效 | 可复用 UI、settings FIFO、模型切换提示和压缩保护 |
| `/provider` | 不存在 | 需要新交互和完整 transition，不只是命令注册 |
| 系统提示词 | base instructions 在 session 初始化时确定；模型切换通过 developer `<model_switch>` 增量注入 | 跨 Provider 是否替换 base prompt 仍无合同 |
| 工具集合 | 每个 sampling step 重建，并按 Provider/模型 capability 过滤 | Provider 真正更新后可复用；当前固定 Provider 使其无法响应热切换 |
| slash commands | 只按 feature、认证和 UI 状态过滤 | 没有 Provider-aware command capability |
| 压缩 | 已按当前 Provider 路由 remote/local；DeepSeek local compact 用 Pro | 跨 Provider 切换缺少 previous-provider 快照与兼容策略 |
| session 历史 | canonical history 较中立，已有 call/output 配对与输入模态清理 | 缺少 OpenAI 专属 reasoning/hosted item 到 DeepSeek 的 wire 投影清理 |
| replay/resume | session metadata 记录单一 Provider，turn context 记录模型但不记录 Provider | 无法准确回放同一 thread 内多 Provider 分段 |
| 模型目录与缓存 | 公共 picker 被 Whale 过滤为 `deepseek-*`；缓存未按 Provider 隔离 | 切到 OpenAI 后目录会错误，且可能串用其他 Provider 的缓存 |

## 2. Provider 与认证

### 2.1 可复用事实

- `ModelProviderInfo` 已承载 endpoint、环境变量凭据、wire API、重试和连接能力：`third_party/codex-cli/codex-rs/model-provider-info/src/lib.rs:98`。
- OpenAI Provider 使用 `requires_openai_auth=true`，认证由 Codex `AuthManager` 管理：`model-provider-info/src/lib.rs:380-415`。
- DeepSeek Provider 已内置 `https://api.deepseek.com`、Responses API 和 `DEEPSEEK_API_KEY`：`model-provider-info/src/lib.rs:418-440`。
- OpenAI 订阅与 OpenAI API 已由 `AuthMode::Chatgpt`、`AuthMode::ApiKey` 区分：`third_party/codex-cli/codex-rs/protocol/src/auth.rs:6-25`。
- OpenAI endpoint 会随认证模式选择 ChatGPT Codex backend 或 `api.openai.com/v1`：`model-provider-info/src/lib.rs:292-310`。
- 浏览器 OAuth、device code 和 OpenAI API Key 登录均已存在：`third_party/codex-cli/codex-rs/cli/src/login.rs:138-233,316-374`。

### 2.2 DeepSeek Key 的现有错位

当前用户文案和真正的运行时凭据来源不一致：

- CLI 把 `whale login --with-api-key` 描述为 DeepSeek Key 入口：`third_party/codex-cli/codex-rs/cli/src/main.rs:518-526`。
- 该入口实际调用通用 `login_with_api_key`，写入 `auth.json` 的 `OPENAI_API_KEY`：`third_party/codex-cli/codex-rs/login/src/auth/manager.rs:958-975`。
- TUI onboarding 同样显示 DeepSeek Key 文案，却提交 `LoginAccountParams::ApiKey`：`third_party/codex-cli/codex-rs/tui/src/onboarding/auth.rs:625-638,778-812`。
- DeepSeek Provider 不读取该 OpenAI auth 字段，只读取进程环境 `DEEPSEEK_API_KEY`：`model-provider-info/src/lib.rs:331-347,418-425`。
- DeepSeek 设置 `requires_openai_auth=false`，默认启动不会进入 OpenAI onboarding：`third_party/codex-cli/codex-rs/tui/src/lib.rs:1906-1920`。

因此，本主题必须先建立 Provider-scoped credential contract，不能把 DeepSeek Key 继续塞进 OpenAI 字段。

### 2.3 认证实现缺口与后续确认结果

盘点时以下问题尚未解决；后续已经由 PRD 的 PD9、PD13、PD15 确认：

- OpenAI 订阅与 OpenAI API 凭据需要安全共存并随时互切，不能由新登录覆盖旧登录。
- DeepSeek Key 按 Provider 安全持久化、可更新，并与 OpenAI 凭据隔离。
- transition 失败必须保持原 Provider 完全不变；具体离线/在线校验层次由工程计划控制。
- `/logout` 只清除当前访问方式，不得清除其他凭据。

## 3. `/provider` 与模型目录

### 3.1 当前命令链路

- slash command enum 中只有 `/model`，没有 `/provider`：`third_party/codex-cli/codex-rs/tui/src/slash_command.rs:12-88`。
- `/model` 只打开模型选择器：`third_party/codex-cli/codex-rs/tui/src/chatwidget/slash_dispatch.rs:293-296`。
- 选中模型后只更新 model、effort 与模型持久化配置：`third_party/codex-cli/codex-rs/tui/src/chatwidget/model_popups.rs:235-287`。
- `thread/settings/update` 的输入只有 model，没有 model provider：`third_party/codex-cli/codex-rs/app-server-protocol/src/protocol/v2/thread.rs:223-276`。

现有 `/model` 的 UI、事件和 FIFO settings 提交可复用，但 Provider 与模型必须作为同一 transition 校验并提交，不能先改 Provider 再异步补模型。

### 3.2 Provider-scoped 目录缺口

- model picker 的 available models 构建后无条件只保留 `deepseek-*`：`third_party/codex-cli/codex-rs/models-manager/src/manager.rs:127-141`、`model_presets.rs:3-18`。
- DeepSeek Flash 被无条件标记为 Whale 默认模型，同一逻辑不能用于 OpenAI。
- `models_cache.json` 没有 Provider identity；源码已有切换 Provider 会串缓存的 TODO：`models-manager/src/manager.rs:481-492`。
- 模型 metadata lookup 主要按 slug 匹配，没有显式 Provider namespace authority：`models-manager/src/manager.rs:639-676`。

必须把目录 client、缓存 key/ETag、默认模型、picker 过滤与 fallback metadata 变成 Provider-scoped。

## 4. Session 运行时边界

### 4.1 代码事实

- `SessionConfiguration.provider` 是具体 `SharedModelProvider`：`third_party/codex-cli/codex-rs/core/src/session/session.rs:74-90`。
- Provider 在 session 初始化时创建一次：`third_party/codex-cli/codex-rs/core/src/session/mod.rs:702-706`。
- 每个 `TurnContext` clone 当前 session Provider，形成 turn 快照：`third_party/codex-cli/codex-rs/core/src/session/turn_context.rs:140-190,922-950`。
- `SessionSettingsUpdate` 没有 Provider 字段：`core/src/session/session.rs:584-601`。
- core `ThreadSettingsOverrides` 同样没有 Provider 字段：`third_party/codex-cli/codex-rs/protocol/src/protocol.rs:469-523`。
- Provider ID snapshot 仍从启动时的 `original_config_do_not_use` 读取：`core/src/session/session.rs:213-270`。
- `Op::ThreadSettings` 与 turn 输入共享 submission queue，可保持调用顺序：`protocol/src/protocol.rs:583-590`、`core/src/session/handlers.rs:604-624`。

### 4.2 直接结论

Provider 切换应只在 turn 边界生效。已捕获的 `TurnContext` 不应在单次 response stream、tool loop、retry 或 compact 中途改变 Provider。切换必须同时更新运行时 Provider、认证、模型目录、模型、提示词、工具/命令投影、压缩策略、持久化和 UI 缓存；任一预检失败时不得留下混合状态。

该 turn 边界后来已由 PRD 的 PD8 直接确认；具体原子提交路径由工程计划约束。

## 5. 系统提示词与动态能力面

### 5.1 系统提示词

- base instructions 的启动优先级是显式 config、历史 SessionMeta、初始模型模板：`third_party/codex-cli/codex-rs/core/src/session/mod.rs:658-713`。
- 结果保存在 `SessionConfiguration.base_instructions`，后续请求持续使用：`core/src/session/session.rs:83-90`、`core/src/session/mod.rs:1309-1320`。
- 当前模型变化不会原子替换 base instructions，而是通过 `<model_switch>` developer fragment 注入新模型说明：`core/src/context/world_state/model.rs:6-59`、`core/src/session/world_state.rs:42-82`。
- `PreviousTurnSettings` 只记录 model、comp hash、realtime 状态，没有 Provider：`core/src/session/mod.rs:259-271`。

跨 Provider 时必须定义 base prompt 的 authority、旧 prompt 的处理、Provider 切换边界消息及缓存前缀失效规则。现有 `<model_switch>` 是可复用 seam，但不足以表达 Provider/auth/wire 变化。

### 5.2 工具集合

- 工具 router 在每个 sampling step 重新构建：`third_party/codex-cli/codex-rs/core/src/tools/spec_plan.rs:121-176`。
- Provider capability 已覆盖 namespace tools、image generation、web search、external web access、remote compaction：`third_party/codex-cli/codex-rs/model-provider/src/provider.rs:55-77`。
- DeepSeek 已禁用 namespace/image generation/remote compact：`model-provider/src/provider.rs:338-354`。
- hosted search、namespace、image generation 等会结合 Provider、模型、feature 和认证过滤：`core/src/tools/spec_plan.rs:549-655`。

因此工具装配骨架可复用；核心缺口是 Provider 不可变，以及 capability 粒度还不能表达 slash commands、history wire compatibility、prompt policy 和完整 compact policy。

### 5.3 可用命令

- 命令过滤只接受 feature、认证/UI 和 side conversation flags：`third_party/codex-cli/codex-rs/tui/src/bottom_pane/slash_commands.rs:56-81`。
- 当前没有 Provider capability 输入；`/usage` 仅按 Codex backend auth 隐藏，`/logout` 没有 Provider 语义。

需要为命令建立 Provider-aware availability contract，并同时约束 popup 可见性和直接键入后的错误行为。

## 6. 压缩与上下文迁移

### 6.1 可复用的模型切换保护

- 每个真实 turn 保存上一模型与 `comp_hash`：`third_party/codex-cli/codex-rs/core/src/session/turn.rs:273-280`。
- comp hash 变化时，会在新 turn 前尝试用上一模型压缩：`core/src/session/turn.rs:1081-1126`。
- 切到更小窗口且当前历史超限时，也会 pre-turn compact：`core/src/session/turn.rs:1129-1175`。
- 手动和自动压缩会按当前 Provider capability 选择 remote 或 local：`core/src/tasks/compact.rs:35-77`、`core/src/session/turn.rs:1206-1255`。
- DeepSeek local compact 可换用 `deepseek-v4-pro` 采样：`core/src/compact.rs:243-267`。

### 6.2 跨 Provider 的明确缺口

- previous turn 没有 Provider/auth 快照；“用上一模型 compact”在跨 Provider 时可能错误地使用新 Provider client。
- 旧 Provider 凭据已失效时，没有降级或取消切换合同。
- OpenAI remote compact 与 DeepSeek local compact 的 checkpoint 兼容性没有 transition gate。
- 新 Provider 的 context window、auto compact threshold、compact prompt 与费用策略未被原子重绑定。

## 7. 历史、恢复与审计

### 7.1 已有基础

- canonical history 使用 `ResponseItemEnvelope`，发送前会修复 call/output 配对、清除孤儿 output，并按目标模型输入模态处理不兼容媒体：`third_party/codex-cli/codex-rs/core/src/context_manager/history.rs:155-214,440-455`、`context_manager/normalize.rs:21-215`。
- compaction replacement history、window IDs 和 world-state reference 可持久化并在 resume 重建：`core/src/session/rollout_reconstruction.rs:320-436`。
- start/resume/fork 协议已经可以在创建运行时时指定 Provider，但活跃 thread settings 不能更新 Provider。

### 7.2 仍需补齐

- 非 OpenAI 请求没有完整清理 OpenAI encrypted reasoning、provider-hosted web/image 等历史 item：`third_party/codex-cli/codex-rs/core/src/client.rs:1083-1096,1139,1158-1174`。
- `TurnContextItem` 与 `PreviousTurnSettings` 没有每 turn Provider identity。
- SessionMeta 只能表达单一 session Provider，无法准确归属切换前后的请求、token、费用与错误。
- resume、fork、rollback、subagent 继承和 replay 尚无“最后一次成功 Provider transition”语义。

建议保留 canonical rollout 事实，只为目标 Provider 生成兼容 wire projection；不要为了适配新 Provider 破坏原始历史。

## 8. 已纳入工程计划的实现门禁

以下盘点建议已由产品决策覆盖并纳入 `plan.md`；工程执行仍需逐阶段 rebase：

1. 将三项用户选择建模为两个 wire Provider、三个访问配置：OpenAI + ChatGPT auth、OpenAI + API auth、DeepSeek + DeepSeek credential。
2. 新增原子 `ProviderTransition`，禁止在单次 active turn/tool loop 内切换。
3. transition 预检 Provider、credential、catalog、default model、prompt、tools、commands、compact 和 history migration；失败时保持原状态。
4. 持久化 from/to Provider、model、非敏感 auth kind、policy versions 与 migration verdict，禁止记录 Key。
5. 为模型目录与缓存先补 Provider identity，再开放 OpenAI picker。
6. 设计目标 Provider wire sanitizer，覆盖 encrypted reasoning、hosted items、媒体和 tool call/output。
7. 将 UI 可见状态、core authority、rollout/replay 和 telemetry 绑定到同一次 transition commit。

## 9. 必测矩阵

- OpenAI 订阅 ↔ OpenAI API；OpenAI → DeepSeek → OpenAI。
- 空白 Key、错误 Key、取消输入、验证超时、切换失败回滚。
- active turn、排队输入、stream retry、tool continuation、manual/auto compact 期间请求切换。
- 历史含 encrypted reasoning、function/custom tools、provider-hosted web/image、媒体、pending/aborted call。
- 较小 context window、不同 comp hash、已有 compact checkpoint。
- resume、fork、rollback、clear、subagent 继承及最后有效 Provider 恢复。
- Provider-scoped model cache、ETag、默认模型与 picker 不串源。
- prompt、工具、命令与压缩策略在切换后的第一个 turn 同时生效。
- rollout/telemetry/cost 能按 turn 还原 Provider，且任何日志与事件均不含 Key。

现有可复用测试入口包括：

- `core/tests/suite/model_switching.rs`
- `core/tests/suite/compact.rs`
- `core/tests/suite/compact_remote.rs`
- `core/src/context_manager/history_tests.rs`
- `core/src/session/rollout_reconstruction_tests.rs`
- `app-server/src/request_processors/thread_processor_tests.rs`

本盘点未运行真实模型请求，也未产生费用。
