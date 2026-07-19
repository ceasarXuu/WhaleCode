# R7 精简 Control 与同响应 Patch 修复方案

## 1. 状态

```text
Status: Approved / Implementation In Progress
Scope: shared taskspace_control schema and response tool sequence
Compatibility: none
Projection policies: unchanged
Canonical Map/Event Store: unchanged
```

## 2. 根因约束

provider probe 已证明：同一 response 生成多个工具调用本身不是问题；最小 control 与 direct patch
可以达到 6/6 顺序正确和 6/6 patch 正文一致。失败来自过载的 `taskspace_control` 同时暴露状态机、
多分支 lifecycle、普通工具 schema 和 patch 正文。

因此修复不能只移动 patch 字段，也不能让 Runtime 修复 JSON。TaskSpace control 必须重新收敛为状态工具，
普通工具恢复为 provider 顶层 tool calls。

## 3. Provider 合同

`taskspace_control` 不再嵌套普通工具名称、参数或 patch 正文。需要同响应继续工作时只声明：

```json
{"continuation":"next_tool"}
```

或：

```json
{"continuation":"next_apply_patch"}
```

- `next_tool`：下一顶层调用必须存在，且不能是另一个生命周期 control。
- `next_apply_patch`：下一顶层调用必须是 unnamespaced `apply_patch`。
- `initialize_map`、`transition_node(bind)`、`complete_then_continue` 继续强制要求 continuation。
- `mutate_graph` 保持 continuation 可选。
- terminal、read、expand 和非 bind transition 不允许 continuation。
- 旧 `actions`、`patch_then_actions` 和 nested action payload 直接删除，不保留兼容 parser。

典型 response：

```text
taskspace_control(complete_then_continue, continuation=next_apply_patch)
apply_patch(...)
exec_command(test)
```

仍然只消耗一次 provider request。

## 4. Runtime 底线

完整 response 在执行任何工具前做一次语义无关 preflight：

1. 一个 response 最多一个顶层 `apply_patch`。
2. continuation 声明必须由紧邻的顶层工具满足。
3. `next_apply_patch` 对应的 function arguments 必须是合法 JSON，且包含字符串 `input`。
4. 任一 preflight 失败时，整个 response `executed_tool_call_count=0`，每个 call id 都收到明确失败输出。
5. 非法 `taskspace_control` 参数仍由 control parser 报错；随后调用因 prior failure 跳过。

Runtime 不选择 next tool、不移动调用、不补调用、不修改 patch，也不根据任务语义判断 continuation 是否合理。

## 5. 执行顺序

sequence executor 按 provider 给出的顺序机械分段：

```text
taskspace_control -> control barrier
apply_patch       -> patch barrier
other tools       -> consecutive parallel segment
```

control 失败则 patch 和后续调用跳过；patch 失败则后续调用跳过。patch barrier 保证测试等后续动作不会和
文件修改并行。状态交接仍在 patch 前提交，与当前 nested carrier 的执行顺序一致。

## 6. 反馈与日志

顶层 patch 使用原生工具反馈，不再经 control aggregate 改写。新增或保留以下稳定日志：

```text
tool.response_preflight_rejected
taskspace.response_continuation_validated
tool.control_barrier_started/completed/failed
tool.patch_barrier_started/completed/failed
tool_response_sequence_call_skipped
```

日志只记录 call id、tool name、continuation kind、数量和 reason code，不记录 patch 正文。

## 7. 工程步骤

1. 精简 `taskspace_control` schema 和 typed args，删除 nested action 类型及执行接口。
2. 将 manifest 收敛为纯顶层调用清单，并记录 continuation requirement。
3. 扩展 full-response preflight 和结构化失败结果。
4. 把 `taskspace_control` 与 `apply_patch` 都设为有序 barrier，删除 nested batch executor。
5. 升级 working protocol、observer 和固定测试 fixture。
6. 运行 Rust unit/integration、PowerShell observer、自包含 provider probe。
7. 构建 Docker binary，运行 simple/complex 与同期 Standard；检查 request、token、cache、Map 和 patch trace。

## 8. 验收门禁

```text
schema 中 patchAction / ordinaryAction / patch_then_actions == 0
同响应 control -> patch -> actions 顺序测试 100% 通过
缺失/错序 sibling 在任何状态提交前 100% 拒绝
一个 response 最多一个 patch
patch 原始反馈逐字节进入 provider context
simple/complex public + hidden validator 通过
真实 patch handoff 不增加必然 provider request
无 projection policy、Map 状态或事件 hash 分叉
```

provider 自主遗漏 sibling 仍可能触发一次重试，这是可观测的模型能力风险，不允许 Runtime 静默补救。
若真实样本中该风险抵消或超过正文修复收益，本方案不得判为完成。

## 9. 依据

- `12-r7-nested-patch-control-root-cause.md`
- `benchmarks/taskspace/r7/nested-patch-control-probe-result.json`
- `benchmarks/taskspace/r7/sibling-patch-sequence-probe-result.json`
- [DeepSeek Tool Calls Guide](https://api-docs.deepseek.com/guides/tool_calls)
- [DeepSeek Create Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion/)
- [JSON Schema Combining](https://json-schema.org/understanding-json-schema/reference/combining)
