# build-R4 TaskSpace tools 调用链路专项立项

> build-R4 专项解决 tools 调用链路在 TaskSpace 中的性能和质量问题。R4 不把问题收窄为
> 某一次 `apply_patch` 用法失败，而是把 tool intent、runtime 执行、model-visible feedback、
> TaskSpace map、active projection、cache layout、样本现场和门禁证据作为一条完整链路处理。

## 0.1 元数据

```text
Created: 2026-06-30
Updated: 2026-06-30
Version: v0.0.5 build-R4
Status: Draft
Owner / Responsible: WhaleCode core runtime
Related Systems: TaskSpace runtime, tool runtime, action-contract transport,
  context compiler/cache planner, active projection, benchmark harness,
  CodeMode, MCP, multi-agent tools
Related Links:
  docs/v0.0.5/build-R3/09-r3-engineering-closeout.md
  docs/v0.0.5/build-R3/10-light-effect-experiment.md
  docs/v0.0.5/build-R4/04-benefit-gates-and-public-sample-acceptance.md
  coe/2026-06-20-02-00-taskspace-tool-call-history.md
  coe/2026-06-27-00-48-r3-btier-regressions.md
Risk Level: High
Plan Type: Full
AI Agent 推理程度: high
```

## 0.2 背景

R3 已经把 TaskSpace 的上下文编译器、cache planner、active replacement proof、graph closeout、
timing attribution 和 start gate 脚手架收敛到工程可验状态。但轻量级真实样本显示：

1. TaskSpace 在部分样本中知道正确方向，却没有把 patch 落盘。
2. 某些 action-contract 内部 tool 失败没有以 standard 模式等价的方式反馈给模型。
3. 部分样本出现长时间循环、policy violation 重复、rollout 日志膨胀和 wall time 明显放大。
4. 历史上出现过 tool call / tool result 配对不完整导致 provider 协议失败的现场。

这些现象不能继续靠单点补丁处理。R4 需要建立 tools 链路的结构化模型、覆盖矩阵和样本驱动门禁。

## 0.3 问题定义

当前 TaskSpace tools 链路存在下列系统性风险：

| 风险 | 说明 | 影响 |
|---|---|---|
| 反馈链路不唯一 | direct tool、action-contract internal tool、compile rejection、CodeMode nested tool、multi-agent/MCP tool 走不同路径 | 修复一个路径不能证明全局正确 |
| 语义可能被改写 | tool stderr、exit code、target path、失败类型可能在 preview、summary、projection 中丢失 | agent 无法基于真实失败纠错 |
| map 和 provider payload 不一致 | TaskSpace map 记录失败摘要，但 provider-visible history 未必含同等细节 | map 说失败不等于 agent 下一轮能看到可执行原因 |
| projection 缺少显式理由 | large raw output、legacy output、TaskSpace shadow projection 被 omit 时缺少统一审计 | 可能误删 tool feedback 或破坏 tool-call pairing |
| 性能问题混入质量问题 | 长循环、日志膨胀、重复 policy violation 会放大 wall time/token 成本 | 成本问题不能靠硬上限解决，需要链路收敛 |

## 0.4 R4 目标

| Goal | Expected Benefit | Verification |
|---|---|---|
| 建立 tools 调用链路静态模型 | 知道每类 tool intent/result 实际经过哪些模块 | `01-static-tool-chain-map.md` 覆盖 direct、action-contract、nested、MCP、多 agent、large output |
| 对齐 standard 的 tool feedback 语义 | TaskSpace 可增加 map/projection，但不能篡改 standard 可见反馈 | 每类 tool result 有 provider-visible payload proof 或显式 ref proof |
| 修复 action-contract 内部 tool parity | `apply_patch`、shell/test、parse/policy rejection 都有一致反馈 | 复跑 `count-call-stack` 等失败样本，下一轮 payload 含具体失败原因 |
| 建立真实样本账本 | 用历史现场驱动设计，不只靠单元测试 | `02-field-evidence-and-sample-ledger.md` 记录样本、症状、证据路径和设计结论 |
| 控制性能和日志膨胀 | 消除无意义循环和 uncontrolled rollout bloat | large-output 样本不再 900s timeout / 491MB rollout |
| 保持 DeepSeek cache hit | tools feedback 修复不能破坏 R3 cache-friendly layout | request 2+ cache hit 维持 `>= 0.95`，stable prefix 非预期变化为 0 |

## 0.5 非目标

```text
不重新定义 TaskSpace 产品愿景。
不重新引入 request/session 硬预算上限。
不把工具错误静默吞掉或转成模糊自然语言。
不使用关键词模板绕过模型生成。
不把单个 sample pass 伪装成 E3 aggregate utility pass。
不为了 map 简洁而删除 replay/debug 所需的完整 tool trace。
```

## 0.6 R4 阶段总览

| Phase | Theme | Main Output | Exit Gate |
|---|---|---|---|
| R4-A | Inventory and static chain model | tools 链路静态图、分支推理和风险矩阵 | 每类 tool path 有 owner、输入、输出、projection 规则 |
| R4-B | Field evidence mining | 历史 sample ledger 和真实现场分类 | 至少覆盖 solved/wrong/timeout/invalid-history 四类现场 |
| R4-C | Tool feedback contract and instrumentation | 标准反馈契约、trace 事件、provider payload proof | tool result 语义字段可从 runtime 追到 provider payload/map |
| R4-D | Action-contract internal tool parity | 内部 apply_patch/shell/test/rejection 与 standard 对齐 | 复跑已知 wrong sample，失败反馈可见且可纠错 |
| R4-E | Projection/output-ref/performance safeguards | projection reason、large-output ref、log bloat 控制 | large-output 样本不再无界日志膨胀 |
| R4-F | Multi-agent/MCP/CodeMode coverage | 非 direct tools 覆盖矩阵和必要修复 | 不再有未分类 tool result path |
| R4-G | Benchmark gates and benefit validation | known-bad 样本和 10 个公开 benchmark 综合验收 | correctness、wall time、tool count、token/cache、tool feedback 均有真实证据 |
| R4-H | Closeout | 工程收口文档和后续 E3 入口 | 文档、代码、测试、样本证据一致 |

## 0.7 当前已知最高优先级问题

| Priority | Problem | Evidence |
|---|---|---|
| P0 | action-contract 内部 tool 失败反馈未证明等价 standard | `count-call-stack` 中 `apply_patch verification failed` 未以普通 tool result 清晰回灌 |
| P0 | provider-visible history projection 仍有 tool call/result pairing 风险 | 历史 CoE 记录 invalid tool-call history |
| P1 | direct tool error preview 与 success preview 使用不同语义来源 | `parallel.rs` success 走 `ToolOutput::to_response_item`，error 走 `action_map_tool_error_preview` |
| P1 | large output 和 repeated policy violation 可能导致日志膨胀 | `large-output-ref-smoke` 出现 900s timeout、rollout 约 491MB |
| P1 | action-contract parse/policy rejection 是 recovery text，不是统一 tool-like feedback | `multi-file-order-pipeline` 中出现 strict JSON / unknown action loops |
| P2 | CodeMode nested tool 和 multi-agent/MCP tool map attribution 不完整 | `ToolCallSource::CodeMode` 和 multi-agent tools 被 direct attribution 排除 |

## 0.8 R4 总门禁

R4 完成不能只说“没有明显缺口”，必须满足：

1. 静态链路矩阵中没有 `unknown` 或 `unowned` path。
2. 每个高风险 path 至少有一个单元/fixture 测试和一个 provider-visible proof。
3. 至少复跑四类真实样本：`single-file-fast-fix`、`count-call-stack`、`multi-file-order-pipeline`、`large-output-ref-smoke`。
4. 对历史 invalid tool-call history 有回归 fixture 或等价协议检查。
5. correctness 和性能收益必须分开报告：不能用 solved 掩盖 wall time/token/log bloat 回归。
6. cache hit 不能因 tool feedback 修复倒退，request 2+ hit rate 目标为 `>= 0.95`。
7. 每个 phase 必须证明具体工程收益，证据至少包含 baseline、after、测量方法、artifact 路径和 pass/fail 结论。
8. R4-G 必须从公开 benchmark 选择 10 个考验 tool 调用能力的真实样本，记录 source URL、版本、commit、task id、选择理由，并跑一轮综合验收。
9. 工程变更完成后写入 closeout 文档，记录真实收益、未收敛风险和是否可进入后续 E3。
