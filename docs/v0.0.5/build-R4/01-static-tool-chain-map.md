# R4 静态 tools 链路图和分支推理

> 本文记录 TaskSpace tools 调用信息从 agent intent 到 provider 下一轮 payload 的静态路径。
> 目标是先证明“信息怎么走”，再决定“哪里需要修”。R4 不能只修当前暴露的样本路径。

## 1.1 目标链路

R4 期望的最佳路径如下：

```text
Agent tool intent
  -> standard tool execution / standard validation
  -> canonical ToolFeedbackEnvelope
  -> ResponseInputItem / provider-visible item
  -> conversation history append
  -> provider projection decision with explicit reason
  -> TaskSpace map record using same semantic payload or stable ref
  -> trace event and benchmark artifact
  -> next agent request can see the exact actionable feedback
```

TaskSpace 可以增加 map、node、edge、summary、projection 和 cache planner，但不应该在默认路径中重写
tool result 的核心语义。核心语义至少包括：

```text
tool name
call id
source path / target path
arguments summary
exit status / structured status
stdout preview or output ref
stderr preview or error ref
failure type
provider-visible payload id/hash
TaskSpace node id / action id
projection decision and reason
```

## 1.2 已识别代码入口

| File | Function / Area | Role | Static Risk |
|---|---|---|---|
| `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs` | `handle_tool_call` / `handle_tool_call_with_source` | direct tool 执行入口 | success 和 error 记录路径不同 |
| `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs` | `record_taskspace_tool_result` | direct tool 写入 TaskSpace map | 只覆盖 `should_attribute_taskspace_tool=true` 的工具 |
| `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs` | `should_attribute_taskspace_tool` | 决定是否写入 map | 排除 non-direct、multi-agent control、taskspace control |
| `third_party/codex-cli/codex-rs/core/src/tools/context.rs` | `ToolOutput::to_response_item` | standard model-visible feedback 抽象 | 应成为 TaskSpace 反馈语义源 |
| `third_party/codex-cli/codex-rs/core/src/tools/context.rs` | `tool_output_model_visible_preview` | 从 response item 抽取 preview | success path 已用，error/internal path 未全覆盖 |
| `third_party/codex-cli/codex-rs/core/src/session/turn.rs` | action-contract prompt / response helpers | action-contract 内部 tool call/result 合成 | 可能绕过 standard tool result 入口 |
| `third_party/codex-cli/codex-rs/core/src/session/turn.rs` | `provider_visible_history_action` | active projection 过滤/保留 | 可能 omit tool result 或破坏 pairing |
| `third_party/codex-cli/codex-rs/core/src/session/turn.rs` | `is_large_raw_tool_output` | large output 判断 | 只覆盖部分 output 类型 |
| `third_party/codex-cli/codex-rs/core/src/tools/code_mode/mod.rs` | `call_nested_tool` | CodeMode nested tool | 使用 `ToolCallSource::CodeMode`，不走 direct map attribution |
| `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_common.rs` | `tool_output_response_item` | multi-agent output 包装 | JSON text wrapper 与 direct path 语义未完全统一 |

## 1.3 分支推理矩阵

| Branch | Current Static Observation | Required R4 Decision |
|---|---|---|
| direct tool success | 已可通过 `ToolOutput::to_response_item` 生成 model-visible preview 并写 map | 保留，但补 payload hash / node attribution proof |
| direct tool execution error | 2026-06-30 已改为从 `failure_response_for_error` 的 standard `ResponseInputItem` 提取 map preview | 继续补 payload hash / envelope trace，确认 stderr/exit/path 不丢 |
| direct tool fatal / abort | 可能不进入普通 response item | 明确是否 provider-visible、是否 map-visible、是否 terminal |
| action-contract internal apply_patch | 由 `turn.rs` 合成 tool call/output，执行失败现场显示反馈不稳定 | 必须与 standard apply_patch failure 语义等价 |
| action-contract shell/run_test | 可能被 node policy 拦截或作为 validation/recovery text | 区分 policy rejection、tool execution failure、test failure |
| action-contract JSON parse rejection | 当前像 prompt correction/recovery 文本 | 需要 tool-like structured feedback，避免模型无视失败继续提交 |
| node policy violation | 例如 `unknown:read_file`、`inspect_code_context:run_test` | 反馈应包含当前 node 可用 action、原因、下一步可执行选择 |
| active projection absent | history 全量进入 payload 的风险较高 | 需要保证 pairing 正确但允许作为 fallback |
| active projection present | legacy TaskSpace output / large raw output 可被 omit | 必须有 explicit projection reason 和 ref，并测试 pairing |
| small output | 可直接进入 payload/map preview | preview 与 standard output 内容一致 |
| large output | 可能走 large raw omission 或 ref | ref 必须可检索，summary 不能替代 actionable error |
| CodeMode nested tool | `ToolCallSource::CodeMode` 被 direct attribution 排除 | 决定是否归入当前 node 子 trace，至少不能丢 provider-visible feedback |
| multi-agent tool | spawn/wait/close 等被 map attribution 排除 | control 类可排除，但 agent 结果和错误需要明确归属 |
| MCP / tool-search output | `to_response_item` 支持，但 recent-output selection 未全覆盖 | 补 coverage 或明确非 TaskSpace path |

## 1.4 语义丢失检查点

R4 静态审计和实现时必须逐点检查：

1. `call_id` 是否跨 synthetic call、runtime call、provider output、map record 保持可追踪。
2. `stderr` 是否被 summary 截断到失去可执行原因，例如缺少目标路径、缺少 verification failed。
3. `stdout` 大输出是否用 ref 保存，并在 agent 需要时可渐进暴露。
4. tool `exit_code`、timeout、abort 是否与业务失败区分。
5. `apply_patch` 失败是否区分语法错误、路径错误、上下文不匹配和权限错误。
6. `run_test` 失败是否区分测试断言失败、命令找不到、环境缺失和 policy 不允许。
7. active projection 是否同时处理 tool call 和 tool result，不能只 omit 一侧。
8. TaskSpace map 摘要是否基于同一个 canonical feedback，而不是独立手写摘要。

## 1.5 管理机制缺口

当前缺口不是“缺一两个 if 分支”，而是缺少 tools 链路治理：

| Governance Area | Required Mechanism |
|---|---|
| Path ownership | 每类 tool result path 必须有 owner phase 和测试名 |
| Semantic contract | 定义 `ToolFeedbackEnvelope` 或等价 contract，统一 provider/map/projection 使用 |
| Projection audit | 每次 omit/summary/ref 都记录 reason、source item id、target ref |
| Replay safety | replay 能根据 trace 重建 provider-visible tool feedback |
| Benchmark observability | pair report 增加 tool feedback loss、projection omit、large-output ref、policy-loop 指标 |
| Regression fixtures | known-bad samples 固化成轻量 fixture，避免只靠人工读 target 目录 |

## 1.6 R4-A 退出条件

R4-A 完成时需要产出：

1. tool path coverage table：direct、action-contract、nested、multi-agent、MCP、large-output 全覆盖。
2. 每个 path 标记 `canonical`、`needs-fix`、`intentionally-excluded` 或 `out-of-scope`。
3. 每个 `intentionally-excluded` path 有工程理由和回归测试。
4. P0/P1 path 有对应 R4 phase owner。
5. 至少一个静态测试或脚本能防止未来新增 tool path 未登记。

2026-06-30 工程化补充：

```text
Manifest:
  docs/v0.0.5/build-R4/r4-tool-path-coverage.json
Validator:
  scripts/taskspace-benchmark/test-r4-tool-path-coverage.ps1
Gate integration:
  scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1
Gate name:
  r4_tool_path_coverage
```

该 manifest 把 R4-A 静态链路矩阵变成机器可读门禁。validator 会检查：

1. path id 唯一。
2. status 只能是 `canonical`、`needs-fix`、`intentionally-excluded`、`out-of-scope`。
3. 不允许 `unknown` 或 `unowned`。
4. P0/P1 path 必须绑定 `R4-A` 到 `R4-H` 中的 owner phase。
5. 每个 source anchor 必须能在当前源码中找到。
6. 每个 path 必须声明 required semantics 和 required evidence。
7. `intentionally-excluded` path 必须写明 rationale 和 test。

这只能关闭 R4-A 的“管理机制/覆盖登记”缺口，不能代替 R4-C 到 R4-F 的 runtime 语义修复。
