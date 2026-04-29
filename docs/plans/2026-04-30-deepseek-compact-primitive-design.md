# DeepSeek Compact Primitive 设计方案

日期：2026-04-30

## 结论

本方案不重新设计 compact 系统。Whale 继续复用 Codex base 已有的 compact 生命周期、历史替换、事件、token 重算、resume/replay 和 TUI 展示链路，只在 compact 执行策略选择点新增 DeepSeek 专属策略，并把 compact 明确登记为 Whale 的 PrimitiveModule。

第一阶段目标：

- OpenAI/Azure Responses provider 继续走 Codex remote compact。
- DeepSeek provider 走新增 `DeepSeekCompact` 策略，固定用 `deepseek-v4-pro`。
- 其他 provider 继续走 Codex local fallback compact。
- 所有触发场景统一覆盖：手动 `/compact`、自动 pre-turn compact、自动 post-turn/mid-turn compact、模型窗口切换触发 compact、主 agent、子 agent。
- TUI 默认 footer 显示轻量上下文占用：`deepseek-v4-pro high (87k/1M)`。
- `/status` 在 `Context window` 下方新增 `Auto compact threshold` 行，例如 `755K`。
- prompt 基本沿用 Codex prompt，只补充少量 Whale 状态保真要求。

## 设计边界

### 做什么

1. 把 compact 定义为一等 PrimitiveModule：有触发策略、执行策略、输入输出契约、事件、指标和测试。
2. 最小侵入 Codex base：把现在的 `remote vs local fallback` 二分选择，扩展成 `remote vs deepseek vs local fallback`。
3. DeepSeek compact 只改变 compact 使用的模型和 prompt profile，不改变历史替换算法。
4. 上下文展示只增加必要信息，避免 footer 信息过载。

### 不做什么

1. 不新建第二套 conversation memory 或 summary store。
2. 不改变 Codex 已有 compact replacement history 语义。
3. 不把 compact 做成普通 slash command 的局部功能。
4. 不引入未经验证的大量 Whale 特性 prompt。
5. 不在 footer 展示阈值、百分比、价格、cache 等额外字段。

## 当前 Codex Base 机制

### 执行策略

Codex 现在有两个 compact 实现：

- local fallback：`third_party/codex-cli/codex-rs/core/src/compact.rs`
  - `should_use_remote_compact_task` 只检查 provider 是否支持 remote compaction。
  - `run_inline_auto_compact_task` 和 `run_compact_task` 都进入同一套 local compact 逻辑。
  - compact 请求本质是一次普通模型调用，最后取 assistant summary 生成 replacement history。
- remote compact：`third_party/codex-cli/codex-rs/core/src/compact_remote.rs`
  - `run_remote_compact_task` 调 Responses compact API。
  - remote 输出会经过过滤和标准化，再安装到 session history。

provider 能力判断在 `third_party/codex-cli/codex-rs/model-provider-info/src/lib.rs`：

- `supports_remote_compaction()` 只对 OpenAI/Azure Responses provider 返回 true。
- DeepSeek provider 是 `WireApi::ChatCompletions`，所以现在会落到 local fallback。

### 触发时机

触发集中在 `third_party/codex-cli/codex-rs/core/src/session/turn.rs`：

- turn 开始前读取 `auto_compact_limit`。
- pre-sampling 阶段：历史 token 已超过当前模型 compact 阈值时触发。
- post-sampling 阶段：本轮响应后 token 达到阈值时判断。
- mid-turn 阶段：如果 token 已达阈值且还需要继续处理 follow-up/pending input，则 inline compact。
- 模型窗口切换：如果切换到更小窗口模型会触发 previous-model inline compact。

DeepSeek V4 模型元数据已在 `third_party/codex-cli/codex-rs/models-manager/models.json` 中配置：

- `context_window: 1000000`
- `max_context_window: 1000000`
- `auto_compact_token_limit: 755000`
- `effective_context_window_percent: 90`

所以第一阶段不需要重建触发器，只需要让触发器调用到 DeepSeek 专属 compact 策略。

### 历史替换和 replay

compact 完成后进入 `replace_compacted_history`：

- 写入 `RolloutItem::Compacted`。
- 必要时写入 `RolloutItem::TurnContext`。
- 重算 token usage 并发出 `TokenCount`。

这部分已经覆盖 resume/replay、TUI token 状态刷新和 app-server v2 token usage 通知。DeepSeekCompact 必须复用这条链路。

### 子 agent

子 agent spawn config 会继承父 turn 的 model/provider/reasoning/compact prompt 相关配置，关键入口在：

- `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_common.rs`
- `third_party/codex-cli/codex-rs/core/src/session/review.rs`
- `third_party/codex-cli/codex-rs/core/src/guardian/review_session.rs`

因此 DeepSeekCompact 不需要为子 agent 单独实现一条 compact 流程。只要策略选择发生在 session compact 入口，主 agent 和子 agent 都会覆盖。

## PrimitiveModule 定义

这里的 PrimitiveModule 不是插件市场意义上的外部插件，而是 Whale runtime 的一等能力模块。compact 作为 PrimitiveModule 需要满足以下契约：

```text
CompactPrimitiveModule
  id: "context.compact"
  scope: session + turn + agent
  trigger_policy: manual | pre_turn_auto | post_turn_auto | mid_turn_auto | model_window_switch
  strategy: openai_remote | deepseek_pro | local_fallback
  input_contract: current history + turn context + initial context injection policy
  output_contract: replacement history + compacted item + token usage refresh
  events: started | completed | failed | warning | token_count_updated
  replay_state: RolloutItem::Compacted + optional RolloutItem::TurnContext
```

第一阶段实现重点不是搭一整套 PrimitiveModule 框架，而是在 compact 边界上补齐这些字段、日志和测试，使它从普通函数选择升级为可演进的原语。

## 策略选择

目标选择顺序：

```text
if provider.supports_remote_compaction():
    OpenAiRemoteResponsesCompact
else if provider.is_deepseek():
    DeepSeekCompact(model = "deepseek-v4-pro")
else:
    LocalFallbackCompact
```

理由：

- OpenAI remote compact 是 Codex 已有云端能力，保持原样。
- DeepSeek 没有 Codex `/responses/compact` 云端接口，不能伪装成 remote compact。
- DeepSeek compact 应固定用 pro，不跟随当前 turn 的 flash/pro 选择。
- 其他 provider 继续用 Codex fallback，避免扩大改动面。

`DeepSeekCompact` 的实现应尽量复用 local fallback 的内部流程：

- 使用相同 `InitialContextInjection` 规则。
- 使用相同 `build_compacted_history`。
- 使用相同 `replace_compacted_history`。
- 使用相同 turn item started/completed 事件。
- 只在 sampling model/provider/prompt profile 上分叉。

## Prompt 策略

基线 prompt 继续沿用 Codex `SUMMARIZATION_PROMPT`。Whale 只追加一个短 appendix，要求 summary 保留 Whale 特有状态：

- 当前任务目标、用户明确决策和未决讨论点。
- 已执行/未执行的工具和验证结果。
- 多 agent 场景中的角色、分工、关键结论和冲突点。
- Debug 场景中的 hypothesis/evidence 状态。
- Create 场景中的 scaffold、test、logging 约束状态。

不追加复杂策略、不引入未经验证的格式，不要求模型输出新的 artifact schema。第一阶段只要求 compact summary 不丢 Whale 原语状态。

## 触发场景覆盖矩阵

| 场景 | 当前入口 | 第一阶段要求 |
| --- | --- | --- |
| 手动 `/compact` | `core/src/tasks/compact.rs` | 走统一策略选择，DeepSeek provider 固定 pro |
| pre-turn auto compact | `session/turn.rs` pre-sampling | 走统一策略选择 |
| post-turn auto 判断 | `session/turn.rs` token limit check | 保留 755K 阈值 |
| mid-turn auto compact | `session/turn.rs` follow-up/pending input | 走统一策略选择，并保留 initial context injection |
| 模型窗口切换 | `maybe_run_previous_model_inline_compact` | 走统一策略选择 |
| 主 agent | session compact lifecycle | 默认覆盖 |
| 子 agent | spawn config session lifecycle | 默认覆盖，补测试确认 |
| guardian/reviewer session | guardian review session config | 默认覆盖，补测试确认 |

## UI 展示

### 默认 footer/status line

默认 footer 增加轻量上下文占用，不展示阈值：

```text
deepseek-v4-pro high (87k/1M)
```

设计约束：

- 只显示 used/window，不显示百分比。
- token 数使用 compact formatter：`87k`、`1M`。
- 保持一行内展示，不新增 footer 高度。
- 当 token usage 暂不可用时，退化为现有 `model reasoning` 展示，不显示空括号。

当前相关入口：

- 默认 status line items：`third_party/codex-cli/codex-rs/tui/src/chatwidget.rs`
- status line item 渲染：`third_party/codex-cli/codex-rs/tui/src/chatwidget/status_surfaces.rs`
- footer context line：`third_party/codex-cli/codex-rs/tui/src/bottom_pane/footer.rs`

建议新增一个组合 item，例如：

```text
model-with-reasoning-and-context
```

并把 Whale 默认 status line 从：

```text
model-with-reasoning, current-dir
```

调整为：

```text
model-with-reasoning-and-context, current-dir
```

这样比把 `context-used` 和 `context-window-size` 两个 item 拼起来更稳，能直接得到用户要求的 `deepseek-v4-pro high (87k/1M)` 样式。

### `/status`

`/status` 继续保留更详细展示。在 `Context window` 下新增一行：

```text
Auto compact threshold: 755K
```

设计约束：

- 有 `model_auto_compact_token_limit` 时显示。
- 没有阈值时显示 `unavailable` 或省略，需要跟现有 status card unavailable 风格一致。
- 不把阈值放进 footer。

当前相关入口：

- `/status` 命令：`third_party/codex-cli/codex-rs/tui/src/slash_command.rs`
- status card：`third_party/codex-cli/codex-rs/tui/src/status/card.rs`
- status snapshot tests：`third_party/codex-cli/codex-rs/tui/src/status/snapshots/`

## 日志和指标

第一阶段需要保留 Codex analytics，并补充 Whale 可观测字段：

- selected strategy：`openai_remote`、`deepseek_pro`、`local_fallback`
- trigger phase：manual、pre-turn、post-turn、mid-turn、model-window-switch
- source model：触发 compact 前的当前模型
- compact model：DeepSeekCompact 固定为 `deepseek-v4-pro`
- tokens before/after compact
- replacement history item count before/after
- failure reason

这些字段优先放在现有 compaction analytics/log event 周围，不另起日志系统。

## 实施步骤

1. 提取 compact strategy selection
   - 把 `should_use_remote_compact_task` 升级为返回 strategy enum。
   - 保持原 OpenAI remote 和 local fallback 行为不变。

2. 新增 DeepSeekCompact strategy
   - 复用 local fallback compact 主体。
   - sampling 时固定使用 `deepseek-v4-pro`。
   - prompt 为 Codex prompt + Whale appendix。
   - 保留 current session 的 provider credential/base URL。

3. 接入所有入口
   - `/compact` task。
   - auto compact。
   - previous-model inline compact。
   - main/sub/guardian session 通过统一入口自然覆盖。

4. 更新 UI
   - 新增默认 status line 组合项。
   - `/status` 新增 auto compact threshold 行。
   - 更新 TUI tests 和 snapshots。

5. 补测试和日志
   - compact strategy unit tests。
   - DeepSeek provider 选择 pro 的请求体测试。
   - manual compact + auto compact integration tests。
   - child agent config compact coverage tests。
   - TUI footer/status snapshot tests。

## 测试计划

必须覆盖：

- OpenAI/Azure provider 仍选择 remote compact。
- DeepSeek provider 选择 `DeepSeekCompact`。
- 非 DeepSeek 且非 remote provider 仍选择 local fallback。
- DeepSeek 当前模型是 flash 时，compact 请求使用 pro。
- DeepSeek 当前模型是 pro 时，compact 请求仍使用 pro。
- 手动 `/compact` 完成后 history 被替换，TokenCount 更新。
- auto compact 达到 755K 阈值后触发。
- mid-turn compact 保留 initial context injection 语义。
- 子 agent 继承 compact prompt，并在达到阈值时走 DeepSeekCompact。
- footer 展示 `deepseek-v4-pro high (87k/1M)`。
- `/status` 在 `Context window` 下展示 `Auto compact threshold: 755K`。

建议测试入口：

- `core/src/compact_tests.rs`
- `core/tests/suite/compact.rs`
- `core/tests/suite/compact_resume_fork.rs`
- `core/src/tools/handlers/multi_agents_tests.rs`
- `tui/src/chatwidget/tests/status_and_layout.rs`
- `tui/src/status/tests.rs`

## 风险和缓解

| 风险 | 缓解 |
| --- | --- |
| DeepSeekCompact 改动 local fallback 后影响其他 provider | 用 strategy enum 保持 local fallback 原行为 |
| compact prompt 追加内容过多导致 summary 噪音 | 只追加短 appendix，不引入新格式 |
| 固定 pro 导致 provider/model override 漏传 | 测试请求体中的 model 字段 |
| footer 信息过载 | 只显示 `(used/window)`，阈值只放 `/status` |
| 子 agent 漏覆盖 | 通过 spawn config 和 integration test 验证 |
| token 重算不准 | 复用 Codex `replace_compacted_history` 后的 TokenCount 链路 |

## 验收标准

实现完成后，以下现象应同时成立：

1. DeepSeek 会话中执行 `/compact`，请求模型是 `deepseek-v4-pro`。
2. DeepSeek flash 会话达到 auto compact 阈值时，compact 仍用 pro。
3. OpenAI/Azure Responses compact 行为不变。
4. 非 DeepSeek provider local fallback 行为不变。
5. compact 后 resume/replay 能看到 compacted history 和 token usage。
6. 默认 footer 显示类似 `deepseek-v4-pro high (87k/1M)`。
7. `/status` 在 `Context window` 下显示 auto compact threshold。
8. 主 agent、子 agent、guardian/reviewer session 的 compact 入口都被测试覆盖。
