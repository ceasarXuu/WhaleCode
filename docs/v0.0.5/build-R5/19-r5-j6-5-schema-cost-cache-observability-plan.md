# R5-J6.5 Tool Schema 成本与缓存观测修复计划

- Created: 2026-07-12
- Updated: 2026-07-12
- Status: In Progress
- Owner: WhaleCode tools / provider wire observability
- Prerequisite: R5-J6 complete
- Blocks: R5-J7 implementation
- Risk: High

## 1. 问题

J6 为修复 nested action 参数语义缺失，将每个 model-visible ordinary tool 的完整参数 schema 嵌入
`taskspace_control`。当前 wire 中同一 schema 物理出现三次：

```text
top-level native tool
initialize_then_actions.actions union
finish_then_actions.actions union
```

`multi-file-order-pipeline` 中每轮非 message payload 从 Standard 的约 21.69 KB 增长到 R5 的约
48.44 KB。精确 nested schema 相比旧 generic nested schema 单独增加约 17.62 KB/request。

同一运行前两轮又出现0缓存命中。根因不是持续 prefix 破坏，而是新 tools hash 首次出现、Req1
`named taskspace_control` 到 Req2 `auto` 的机械形状变化，以及 Req2 在 Req1 完成70ms后立即发出。当前 telemetry
只比较 tools hash 和 messages LCP，错误地把该边界记为 `prefix_preserved=true`。

## 2. 目标与非目标

### 目标

1. 保留 nested action 的完整原工具参数 schema，不恢复 generic arguments。
2. `taskspace_control` 内部只序列化一份 ordinary-action union，init/finish 通过本地 JSON Schema ref 复用。
3. 空 Map 强制 `taskspace_control` 时只暴露该顶层工具；ordinary schemas 由 control 内部唯一承载。
4. Map 初始化后继续同时暴露 native ordinary tools 和 control，不把所有动作强制收编到 control。
5. cache prefix 指标同时检查 tools、tool choice 和 append-only messages。
6. 区分 message prefix、cache shape、zero/partial/full hit、首次 shape warmup candidate 和同 shape 0-hit。

### 非目标

1. 不增加 sleep、预热请求、自动重试或伪造 cache hit。
2. 不保证 DeepSeek best-effort cache 必然命中。
3. 不移除 top-level native tools，不增加 `actions_only` 通用 carrier。
4. 不压缩、摘要或改写工具 schema 语义。
5. 不在本阶段实施 J7 singular patch contract。

## 3. 设计

### 3.1 Schema 去重

在共享 `JsonSchema` 表达中增加最小 `$defs` / `$ref` 支持：

```json
{
  "$defs": {
    "ordinaryAction": { "anyOf": ["每个可见普通工具的精确 schema"] }
  },
  "anyOf": [
    {
      "action": "initialize_then_actions",
      "actions": {"type":"array","items":{"$ref":"#/$defs/ordinaryAction"}}
    },
    {
      "action": "finish_then_actions",
      "actions": {"type":"array","items":{"$ref":"#/$defs/ordinaryAction"}}
    }
  ]
}
```

约束：

- `$defs` 仅属于当前 function parameter schema，不建立跨 tool 外部引用。
- typed parser、ToolRouter 和原参数 schema 保持不变。
- schema 单测必须证明原工具参数可从 ref target 精确恢复。
- provider wire probe 未接受 `$defs/$ref` 时暂停，不回退 generic arguments。

空 Map 是明确机械状态。`tool_choice=taskspace_control` 时，provider 顶层 tools 只发送 control；初始化后的
`tool_choice=auto` 恢复普通 tools + control。该过滤只依赖 hard state，不推断 Agent 下一动作。

### 3.2 缓存观测

`provider-chat-wire-trace-v2` 增加：

```text
cache_shape_hash = hash(tools_hash + tool_choice kind/name)
message_prefix_preserved
tool_choice_preserved
cache_shape_preserved
tool_choice_changed
```

现有 `prefix_preserved` 收敛为完整 cache-shape 口径：

```text
same tools
and same tool choice
and previous messages are an exact prefix of current messages
```

benchmark extractor 增加：

```text
cache_hit_class = unavailable | zero | partial | full
same_cache_shape_seen_before
cache_warmup_candidate = zero hit and shape not seen earlier in this run
same_shape_zero_hit
tool_choice_transition_count
cache_shape_transition_count
```

这些字段只记录机械事实，不判断 provider 故障，也不驱动 runtime 行为。

## 4. 实施阶段

### J6.5-A：Schema 表达和去重

1. 为 `JsonSchema` 增加 `$defs/$ref` round-trip。
2. `taskspace_control` 将 nested action union 放入单一 `$defs`。
3. init/finish actions items 改为本地 `$ref`。
4. 增加 schema fidelity、引用次数和 serialized bytes 回归门禁。

退出条件：exact ordinary action union 在 control 参数中物理出现一次；参数 schema 等价测试通过。

### J6.5-B：空 Map Tool 可见性

1. `TaskspaceNative + named taskspace_control` 只发送 control spec。
2. 初始化后 `auto` 恢复 ordinary + control。
3. 增加 blank/active provider-visible tool set 测试。

退出条件：初始化 request 不再同时发送顶层 ordinary schema；Agent 仍可在 init carrier 中执行 nested actions。

### J6.5-C：Cache Shape Telemetry

1. wire trace 比较纳入 tool choice。
2. 保留独立 message LCP 字段。
3. extractor 输出 warmup、shape transition 和 same-shape zero-hit 分类。
4. performance observer 展示这些计数。

退出条件：`named -> auto` 记录 `first_diff_path=tool_choice`；不得再报告完整 prefix preserved。

### J6.5-D：验证

测试：

- `codex-tools` schema/registry tests；
- provider wire trace Rust tests；
- cost instrumentation PowerShell tests；
- performance observation tests；
- `cargo check`。

样本：

1. `count-call-stack` Standard/R5 Docker paired run。
2. `multi-file-order-pipeline` Standard/R5 Docker paired run。

收益门禁：

- correctness 和反馈完整性不回退；
- control 内 exact nested schema 物理份数从2降到1；
- blank-map 顶层 ordinary schema 份数从1降到0；
- active request 非 message bytes 明显低于 J6 的约48.44 KB；
- cold run Req1/Req2 被标记 warmup candidate，Req2 `tool_choice` transition 可见；
- warm run同 shape首轮命中可被准确报告，但不作为强保证；
- request、input、cached/uncached、payload bytes、Map health完整分账。

## 5. 风险与回退

| Risk | Gate | Fallback |
|---|---|---|
| DeepSeek 不接受 `$defs/$ref` | stable endpoint wire probe | 暂停并重新设计，不恢复 generic arguments |
| ref target 与原 ToolSpec 漂移 | schema equality test | 回退 J6.5-A |
| blank-map 过滤误删 control | provider tool visibility test | 回退 J6.5-B |
| prefix 指标历史口径变化 | schema version升到V2/V3并更新所有 consumers | 同组回退 telemetry commit |
| 为降低 schema 成本收编普通工具 | forbidden scan/design review | 拒绝该方案 |

## 6. 完成矩阵

| Item | Code | Test | Runtime Evidence | Status |
|---|---|---|---|---|
| `$defs/$ref` | `tools/src/json_schema.rs` | round-trip | provider wire | pending |
| single nested union | `tools/src/taskspace_tool.rs` | fidelity/size | tools hash/bytes | pending |
| blank-map tool filtering | `core/src/session/turn.rs` | visibility | Req1 tools count | pending |
| cache shape trace | `core/src/provider_wire_trace.rs` | named/auto comparison | wire trace v2 | pending |
| cache classification | benchmark instrumentation | fixture | cache summary v3 | pending |
| paired benefit | Docker runner | two samples | observation report | pending |

