# IC-01 Provider Section 归因修复结果

- Status: Complete
- Date: 2026-08-17
- Scope: 仅观测链，不改变 Provider payload、Agent context、Tool 协议或 Runtime 行为

## 结论

此前有两个确定性测量缺陷：

1. Responses API 的顶层 `instructions` 被归入 `other_payload`，使约 20KB 的 Base Instructions 看起来像无法解释的附加内容；
2. Rust 已输出 7 个现行 section，但 PowerShell 消费端仍要求旧协议的 `taskspace_control_feedback`，导致最新真实 trace 的
   section summary 被整体标为 `unavailable`。

当前 `provider-wire-section-cost-v2` 将顶层 `instructions` 单独计入 `base_instructions`，并从生产端、汇总端、性能报告端及
fixture 中删除旧 `taskspace_control_feedback`。Chat Completions 的 Base 仍按其真实 wire 形态计入 `system_messages`，不虚构
顶层 `instructions`。

## 边界

- 每个 Provider payload byte 仍只进入一个 section；对象括号、字段分隔符等结构字节保留在 `other_payload`。
- trace 只保存 count、bytes、估算 token 和 hash，不保存 Base 或消息原文。
- v1 历史 trace 不伪装为 v2；后续 IC-05 只能使用其原有粗粒度边界，无法追溯的细分明确记为 unavailable。
- `base_instructions_identity` 仍是身份观测，不与 `base_instructions` bytes 重复相加。

## 验证

| 验证 | 结果 |
|---|---|
| `cargo test -p codex-core provider_wire_sections --locked` | 12 passed |
| `test-cost-instrumentation.ps1` | passed |
| `test-performance-observation.ps1` | passed |
| section bytes 与完整 payload bytes 对账 | exact |
| Agent/Provider 行为变化 | none，未执行真实 Whale Agent run |

## 后续

IC-02 继续把当前过粗的 `natural_history` 按 wire item 的结构类型拆分；在该步骤完成前，不对历史重复、Map 成本或 outer
call/result 成本作根因结论。
