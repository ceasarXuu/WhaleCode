# DeepSeek 当前三模型与 Responses 能力适配

- 版本：WhaleCode `v0.0.6`
- 状态：已实现，发布候选准备中
- 官方能力快照：2026-08-28
- 实现提交：`521e7730c`
- Vision smoke 修复与验证：`5f907bc28`、`e169c3c0e`
- 产品决策权威：[DeepSeek Responses 能力完整接入 PRD](../../../prd/2026-08-22-deepseek-responses-capability-completion.md)

本文是 `v0.0.6` 的实现与验证记录，不替代 PRD 的产品决策，也不把 DeepSeek 官方会忽略或不支持的字段描述为有效能力。

## 1. 版本结果

`v0.0.6` 将 Whale 的 DeepSeek 支持从“Flash/Pro 可走 Responses 主链”补齐为当前三个官方模型的静态产品目录，并完善 Vision 图片输入、thinking effort、provider 工具边界和 Responses SSE 生命周期。

| 模型 | 输入 | 默认 thinking | 上下文合同 | 产品状态 |
| --- | --- | --- | --- | --- |
| `deepseek-v4-flash` | 文本 | `high` | 1M；755K 自动压缩阈值 | 默认模型 |
| `deepseek-v4-pro` | 文本 | `high` | 1M；755K 自动压缩阈值 | 可选择 |
| `deepseek-v4-flash-vision-exp` | 文本、图片 | `high` | 1M；755K 自动压缩阈值 | 可选择；实验模型 |

三个模型均展示 `none`、`low`、`high`、`max` 四档 thinking effort，均不宣称支持 reasoning summary。Vision 模型是普通 Responses 模型的超集：纯文本任务可以正常使用，只有它会把合规图片作为 `input_image` 保留到请求中。

## 2. 已落地改动

### 2.1 模型目录与选择

- 在内置目录新增 `deepseek-v4-flash-vision-exp`，标记 `text + image` 输入与 original image detail。
- Flash 继续作为默认模型；Pro 与 Vision 均进入模型选择器。
- 公共模型列表继续只展示 `deepseek-*`，不会因远端目录混入其他 provider 模型。
- 三模型统一使用官方 Responses thinking 档位；旧配置中的 DeepSeek `standard` 在发请求时兼容映射为 `high`。

主要实现位于：

- `third_party/codex-cli/codex-rs/models-manager/models.json`
- `third_party/codex-cli/codex-rs/models-manager/src/manager_tests.rs`
- `third_party/codex-cli/codex-rs/core/src/client.rs`

### 2.2 Responses 请求与 Vision

- 内置 DeepSeek provider 继续使用原生 `POST https://api.deepseek.com/responses`，不回退到 Chat Completions。
- Vision 请求保留 data URL 图片、`input_image` 类型与 image detail；CLI 本地图片进入会话历史后仍保持图片语义。
- Flash/Pro 的静态输入模态保持 text-only，避免 UI 把它们描述为视觉模型。
- 通用请求构造器仍可能序列化 DeepSeek 官方声明会忽略的兼容字段；Whale 不把这些字段宣传为已生效功能。

用户用法见根目录 [README](../../../README.md#模型选择)。Vision 示例：

```bash
whale exec \
  -m deepseek-v4-flash-vision-exp \
  --image ./screenshot.png \
  -- "分析这张截图中的问题"
```

`--` 是必要的参数终止符，防止多值 `--image` 参数吞掉后续提示词。Vision 不需要专门的提示词格式；按普通模型描述任务即可，只需确保提示词确实对应所附图片。

### 2.3 Provider 能力边界

DeepSeek provider 现在显式声明：

| 能力 | Whale 行为 |
| --- | --- |
| function tools | 支持 |
| `apply_patch` custom tool | 支持 |
| hosted web search | provider 协议可解析；v0.0.6 不向用户暴露 |
| 并行工具调用 | 支持；DeepSeek 服务端始终启用 |
| namespace tools | 不支持 |
| image generation | 不支持 |
| remote compaction | 不支持；继续使用本地压缩 |

这组静态能力用于阻止 Whale 向 DeepSeek 暴露已知不兼容的 Codex 工具面。v0.0.6 将模型目录的 `supports_search_tool` 保持为 `false`，不在 UI、工具规划或发布说明中向用户宣称 DeepSeek hosted web search；provider 层仅保留协议兼容与事件解析能力，待后续统一模型目录、配置和端到端合同后再开放。

### 2.4 Responses SSE

流式解析补齐了 DeepSeek 当前 Responses 事件形状：

- `response.reasoning_text.delta` 与 `response.output_text.delta` 持续输出 reasoning 和正文；
- `response.function_call_arguments.delta` 与 custom tool input delta 进入工具参数增量；
- function/custom tool 的 `.done` 仅作为确认事件，完整工具调用仍由 `response.output_item.done` 产生，避免重复执行；
- web search 的 `in_progress`、`searching`、`completed` 作为生命周期确认，不重复生成搜索 item；
- 记录非递增 `sequence_number` 的诊断信息，但为兼容 provider 不丢弃事件；
- `response.failed` 和 `response.incomplete` 立即返回错误并结束，不再等待连接 EOF 或 idle timeout。

## 3. 验证证据

### 3.1 离线合同

实现阶段已覆盖以下定向合同：

- 三模型目录可见性、Flash 默认、1M/755K、Vision 模态与四档 thinking；
- DeepSeek Vision 请求走 `/responses` 并保留 `input_image`；
- DeepSeek provider 只暴露约定的 Responses 能力；
- reasoning/text/tool/web-search/usage/failed/incomplete SSE；
- function/custom tool delta 与 done 不产生重复工具执行。

其中 models-manager DeepSeek 定向测试为 3/3，通过 bundled catalog round-trip；Responses SSE 模块测试为 41/41。提交钩子的 TaskSpace zero-base gate 与 cache regression gate 均通过。

### 3.2 真实 API smoke

| 账本记录 | 结果 | 可证明事项 | 证据限制 |
| --- | --- | --- | --- |
| `WAR-20260822-051940-DEEPSEEK-RESPONSES-R3` | batch failed | Flash 工具往返与 Pro 文本样本完成，共 3 个 provider 请求 | Vision 因 CLI 参数解析未发出；runner 未持久化 usage，因此不能作为完整三模型通过证据 |
| `WAR-20260822-053058-DEEPSEEK-VISION-R1` | failed | Vision 请求已真实发出并返回 | 测试素材是书本图标，提示词却按 OpenAI knot logo 判定，属于 oracle 误报；usage 未持久化 |
| `WAR-20260823-001949-DEEPSEEK-VISION-FIX-R1` | passed | Vision 图片语义、最终 marker、usage 与请求计数均通过 | 单样本、单请求，不是质量 benchmark |

最终 Vision 复验使用 1 个 provider 请求、零重试，记录 5,080 input tokens、39 output tokens（其中 31 reasoning tokens），按当时价格快照估算为 `0.01829344 CNY`。完整证据位于：

- [Vision 修复后 summary](../../../benchmarks/deepseek-responses/evidence/WAR-20260823-001949-DEEPSEEK-VISION-FIX-R1/summary.json)
- [Vision 冒烟误报根因与修复](../../../coe/2026-08-23-00-09-deepseek-vision-smoke-marker.md)
- [真实 Whale Agent 运行账本](../../../benchmarks/whale-agent-run-ledger.json)

上述结果应解读为：离线三模型协议合同已覆盖，Vision 已取得完整单次真实通过证据；Flash/Pro 的真实功能调用完成，但当次 harness 没有保留完整 usage，因而三模型 live matrix 尚不能标为完全闭环。

## 4. 当前限制与发布风险

- **Vision 仍是实验模型。** 模型 ID、可用性或服务端行为可能随 DeepSeek 更新而变化，静态目录需要持续跟随官方模型表。
- **不是所有官方参数都生效。** `store`、`previous_response_id`、`conversation`、reasoning summary、verbosity、remote compaction 等能力不得在 Whale 中宣称受支持；部分兼容字段即使出现在通用请求中也会被服务端忽略。
- **非 Vision 模型不会提供视觉理解。** DeepSeek Responses 可能对图片进行服务端降级，Whale 的产品目录仍将 Flash/Pro 标记为 text-only；用户需要视觉语义时必须显式选择 Vision。
- **工具事件依赖最终 item。** 参数 delta 用于流式展示，工具执行以 `response.output_item.done` 的完整调用为准；若 provider 未来不再发送完整 done item，需要重新评估解析策略。
- **hosted web search 暂不开放。** provider 保留协议兼容，但模型静态目录仍标记 `supports_search_tool=false`；v0.0.6 不在工具规划或 UI 中向用户暴露该能力。
- **真实验证深度有限。** Vision 只有一个最小语义样本；Flash/Pro 当次真实 run 缺少可结算 usage。当前证据不代表吞吐、384K 极限输出、600 图上限、长会话或工具压力场景均已验证。
- **协议是无状态的。** DeepSeek 不保存 response/conversation；Whale 必须继续在本地维护历史并在后续 turn 回传必要上下文。

## 5. v0.0.6 发布前检查

- [x] 三个模型进入静态目录，Flash 保持默认。
- [x] Vision 保留图片输入，纯文本也可正常使用。
- [x] thinking effort、provider capability 与 SSE 事件有离线合同测试。
- [x] Vision 单请求真实 smoke 通过且 usage、费用和证据完整。
- [ ] 补一轮能够持久化 usage 的 Flash/Pro 最小真实验证后，再将“三模型 live matrix”标记为完全通过；该动作需要新的真实运行预算授权。
- [x] 2026-08-28 发布准备时复核官方模型与价格页：Flash、Pro、Vision Exp 均在当前模型表中并列为支持 Responses API。
- [x] v0.0.6 用户可见合同固定为不展示、不宣称 hosted web search；provider 只保留协议兼容，后续版本统一能力表达后再开放。

## 6. 关联资料

- [DeepSeek Responses 接入 PRD](../../../prd/2026-08-22-deepseek-responses-capability-completion.md)
- [DeepSeek Responses API 指南](https://api-docs.deepseek.com/guides/responses_api/)
- [DeepSeek Vision 指南](https://api-docs.deepseek.com/guides/vision/)
- [DeepSeek 模型与价格](https://api-docs.deepseek.com/quick_start/pricing/)
- [v0.0.5 原生 Responses 验证报告](../../migration/codex-sync/2026-08-14-u7-deepseek-native-responses.md)
