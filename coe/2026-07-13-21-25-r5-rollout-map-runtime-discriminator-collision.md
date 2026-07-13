# Problem P-001: TaskSpace rollout 中的 Map runtime 事件无法重放
- Status: open
- Created: 2026-07-13 21:25
- Updated: 2026-07-13 21:31
- Objective: 让生产 rollout 中已持久化的 Map checkpoint/delta 能被同一生产 loader 完整读取并稳定重放。
- Symptoms:
  - Docker TaskSpace rollout 原文件包含 checkpoint/delta，`RolloutRecorder::get_rollout_history` 读回后计数为 0。
- Expected behavior:
  - rollout 写入和读取使用同一无歧义 schema；Map checkpoint/delta 不丢失，重放 snapshot hash 稳定。
- Actual behavior:
  - `EventMsg::MapRuntime(MapRuntimeEvent)` 的两个内部 tag 都叫 `type`；loader 先转 `serde_json::Value` 后外层 tag 被覆盖，相关行作为 parse error 跳过。
- Impact:
  - R5 TaskSpace session resume/replay 可能丢失完整 Map runtime 状态；K0 真实 rollout fixture 无法通过。
- Reproduction:
  - 对 `target/r5-k0-docker-billing/.../pair-001/right/artifacts/rollout.jsonl` 运行 K0 captured replay probe。
- Environment:
  - Linux；Rust workspace；branch `whalecode-alpha`；commit `9bbe67d8c50f2ae6b7e4ba472010a44fd3161c36`。
- Known facts:
  - 原文件约 736 KB，机械扫描得到 snapshot checkpoint=1、snapshot delta=68。
  - 生产 loader 读取后 `snapshot_checkpoint_count=0`。
  - 原始 payload 同时包含 `"type":"map_runtime"` 与 `"type":"snapshot_updated"`。
- Ruled out:
  - fixture 缺少 snapshot 事件。
- Fix criteria:
  - captured rollout loader checkpoint/delta 计数与原文件一致；连续 3 次重放 snapshot hash 3/3 稳定；既有 rollout/session 回归通过；benchmark extractor 口径不失效。
- Current conclusion: tag 冲突已证实；直接解析和 Value 中转均无法恢复 Map runtime 事件，必须消除 wire schema 的重复 tag。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: 嵌套 internally-tagged enum 生成重复 discriminator
- Status: confirmed
- Parent: P-001
- Claim: `EventMsg` 与 `MapRuntimeEvent` 都使用 `#[serde(tag = "type")]`，newtype variant 展开后生成两个同名键，使 Map runtime rollout schema 有歧义。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - protocol 定义和原始 rollout 行具有一一对应关系。
- Falsifiable predictions:
  - If true: 序列化 Map runtime 事件会在同一个 payload object 中出现两个 `type` 键。
  - If false: payload 应只有一个 `type` 键，或内层事件位于独立字段/object。
- Diagnostic evidence plan:
  - Prediction or clause under test: 同一个 payload object 出现重复 `type`。
  - Signal: protocol attribute 与真实 JSONL 原始行。
  - Capture method: 读取 enum 定义并检查 snapshot_updated 原始行。
  - Event name or marker:
    - `map_runtime` / `snapshot_updated`
  - Correlation keys:
    - Docker pair `20260713-211306-373/pair-001/right`
  - Differentiates from:
    - fixture 没有 snapshot 事件
  - Supports if:
    - 定义和原始行均出现同名 tag 嵌套。
  - Refutes if:
    - 内外 tag 名不同或内层事件独立嵌套。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready；仍需 H-002 对照决定最小正确修复层
- Next step: 执行直接反序列化与 Value 中转对照。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Value 中转是 loader 丢失外层 tag 的直接触发点
- Status: refuted
- Parent: P-001
- Claim: `load_rollout_items` 的 `str -> Value -> RolloutLine` 中转会合并重复键；若改为 `str -> RolloutLine` 直接流式反序列化，当前 wire 行可以被完整读回。
- Layer: sub-cause
- Factor relation: all_of
- Depends on:
  - H-001
- Rationale:
  - `serde_json::Value` object 不能同时保留两个同名 key，但流式 deserializer 可能按嵌套 enum 访问顺序消费两个 tag。
- Falsifiable predictions:
  - If true: 同一 snapshot 行直接解析成功，Value 中转失败；两者计数不同。
  - If false: 直接解析也失败，必须消除 wire schema 的重复 tag。
- Diagnostic evidence plan:
  - Prediction or clause under test: direct parse 与 Value parse 的 Map runtime 事件计数是否分叉。
  - Signal: 同一真实 JSONL 上两条解析路径的 checkpoint/delta/parse-error 计数。
  - Capture method: K0 captured replay probe 增加只读对照计数。
  - Event name or marker:
    - `taskspace.map_captured_rollout_parse_compared`
  - Correlation keys:
    - Docker pair `20260713-211306-373/pair-001/right`
  - Differentiates from:
    - H-001 需要直接修改 wire schema
  - Supports if:
    - direct parse 恢复 checkpoint=1、delta=68，而 Value loader 为 0。
  - Refutes if:
    - direct parse 同样为 0 或报错。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 诊断后转成永久 round-trip 回归测试，移除临时输出。
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
- Conclusion: refuted；直接流式解析同样无法恢复嵌套事件，loader 中转不是可独立修复的根因。
- Repair design readiness: ready；repair 必须调整 wire schema
- Next step: 将内层 discriminator 改为无冲突字段并更新全链消费者。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 真实 rollout 行包含重复 type
- Related hypotheses:
  - H-001
- Direction: supports
- Type: observation
- Source: `target/r5-k0-docker-billing/subscription-billing-repair/20260713-211306-373/pair-001/right/artifacts/rollout.jsonl`
- Prediction or plan link:
  - H-001 同一 payload object 出现重复 `type`
- Matched signal:
  - `"type":"map_runtime","type":"snapshot_updated"`
- Correlation keys:
  - pair-001/right
- Raw content:
  ```text
  {"type":"event_msg","payload":{"type":"map_runtime","type":"snapshot_updated",...}}
  ```
- Interpretation: 真实持久化格式存在 discriminator 冲突，不是报告器推断。
- Time: 2026-07-13 21:25

## Evidence E-002: protocol 定义复用了 type tag
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `protocol/src/protocol.rs` 的 `EventMsg` 与 `MapRuntimeEvent`
- Prediction or plan link:
  - H-001 enum 定义应产生同名 tag
- Matched signal:
  - 两个 enum 都声明 `#[serde(tag = "type", rename_all = "snake_case")]`
- Correlation keys:
  - none
- Raw content:
  ```text
  EventMsg::MapRuntime(MapRuntimeEvent)
  MapRuntimeEvent::{SnapshotDelta, SnapshotUpdated, ...}
  ```
- Interpretation: 重复 key 来自协议结构本身。
- Time: 2026-07-13 21:25

## Evidence E-003: 生产 loader 丢失全部 snapshot 事件
- Related hypotheses:
  - H-002
- Direction: supports
- Type: reproduction
- Source: K0 captured replay test；`RolloutRecorder::get_rollout_history`
- Prediction or plan link:
  - H-002 Value 中转可能覆盖外层 tag
- Matched signal:
  - `assertion failed: snapshot_checkpoint_count > 0`
- Correlation keys:
  - pair-001/right
- Raw content:
  ```text
  raw rollout: checkpoint=1, delta=68
  loaded history: checkpoint=0, delta=0
  ```
- Interpretation: loader 输出与磁盘事实不一致；尚未完成 direct parse 对照。
- Time: 2026-07-13 21:25

## Evidence E-004: 直接解析同样无法恢复 Map runtime 事件
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: experiment
- Source: K0 captured replay direct parse / Value loader 对照
- Prediction or plan link:
  - H-002 预测 direct parse 可恢复 checkpoint=1、delta=68
- Matched signal:
  - direct parse checkpoint=0；loader checkpoint=0
- Correlation keys:
  - pair-001/right
- Raw content:
  ```text
  loader checkpoint count is 0; direct parse count is 0
  ```
- Interpretation: 消除 Value 中转不能修复该事件；wire schema 必须使用不同的内外 discriminator。
- Time: 2026-07-13 21:31
