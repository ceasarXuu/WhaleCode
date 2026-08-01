# R7 顶层 Patch 与 Required Next Call 验证结果

## 1. 结论

本轮修复没有破坏合并 request。生产 schema 的定向 provider probe 中，6/6 响应都在同一个
provider response 内生成 `taskspace_control -> apply_patch`，6/6 大型 patch 正文逐字节一致。
Docker simple/complex 也都实际出现并成功执行了同响应 control + patch。

但 `required_next_call` 的字段改名没有消除自然 coding 流程中的首次遗漏：两个 TaskSpace 样本都各有
2 次 control 单独返回，收到 preflight 的明确失败后才在下一请求补成 sibling 组合。因此当前应拆分判断：

- 大型 patch 从嵌套 carrier 移回原生顶层工具：通过。
- control、patch、后续普通工具保持同一 provider response：通过。
- 首次 sibling 采用率与 request 效率：未通过，继续作为独立问题。

## 2. 实施边界

生产实现 commit 为 `12e7f8e3e`，观测器 commit 为 `04ac1ba24`。

1. `taskspace_control` 只保留 Map lifecycle 参数和 `required_next_call` 种类声明。
2. patch 正文只进入顶层 `apply_patch`，不再嵌入 control。
3. 完整 response 在任何工具执行前检查 sibling、patch 参数和单 response 单 patch。
4. control 与 patch 都是机械 barrier；普通 sibling 在后续连续段并行执行。
5. Runtime 不补调用、不移动调用、不修 patch、不推断 Agent 下一动作。
6. 旧 `continuation` 和 nested carrier 在产品 parser 中直接拒绝，不保留兼容分支。

## 3. 工程验证

以下测试通过：

```text
codex-tools taskspace_tool: 3 passed
codex-core taskspace_control_args: 17 passed
codex-core tools::sequence: 16 passed
taskspace_terminal_contract: 2 passed
working protocol hash/version: 1 passed
performance observer self-test: passed
sibling patch probe self-test: passed
K0 map budget report self-test: passed
performance observer skill validation: passed
```

扩展运行 `cargo test -p codex-core taskspace --no-default-features` 时，70 个库测试和 2 个 terminal
集成测试通过；`compact_resume_fork::taskspace_manual_compact_rollout_resumes_without_event_sequence_gap`
仍在等待事件时超时。该用例是本轮变更前已可重复的基线失败，本轮未修改 compaction/resume 路径，因此
不把它计作本修复回归，也不把整套过滤测试标为全绿。

`declared_patch_and_follow_up_tools_stay_in_one_valid_response` 固定验证：

```text
taskspace_control barrier
apply_patch barrier
exec_command + read_file parallel segment
```

这条测试直接防止未来把 control、patch 和后续动作拆成多个 provider request。

## 4. Provider Probe

证据：`benchmarks/taskspace/r7/sibling-required-next-call-production-result.json`。

| 指标 | 结果 |
|---|---:|
| HTTP 200 | 6/6 |
| `taskspace_control -> apply_patch` 顺序 | 6/6 |
| control 参数形状合法 | 6/6 |
| patch JSON 合法 | 6/6 |
| patch 正文 exact | 6/6 |
| 第 2 次及以后缓存输入 | 每次 4096/4128 tokens |

该 probe 证明能力层支持合并 request 和原生 patch 保真，但它有明确的“两次调用”指令，不能替代自然
coding 样本的采用率验证。

## 5. Docker 对照

两个 pair 都是完整、有效的一次诊断运行；Standard 与 TaskSpace 使用同一二进制和容器基线，公开与隐藏
验证全部通过。由于每个 scenario 只有 1 次，本表不用于估计总体效用。

| Sample | Mode | 结果 | Requests | Runtime tools | 时间(s) | Input | Cached | Uncached | Output | Req2+ cache | Map N/E | Required call 满足/声明 |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| simple | Standard | solved | 6 | 8 | 19.502 | 42,678 | 40,704 | 1,974 | 1,692 | 94.96% | 0/0 | 0/0 |
| simple | TaskSpace | solved | 11 | 11 | 31.028 | 112,480 | 101,248 | 11,232 | 2,894 | 96.44% | 5/4 | 3/5 |
| complex | Standard | solved | 11 | 17 | 49.041 | 110,380 | 105,600 | 4,780 | 4,694 | 95.48% | 0/0 | 0/0 |
| complex | TaskSpace | solved | 25 | 32 | 104.983 | 501,116 | 490,496 | 10,620 | 11,539 | 97.88% | 5/4 | 4/6 |

TaskSpace 两次运行的 provider prefix 都为 100% 保留；simple 首请求是唯一 zero-cache warmup，未出现
same-shape zero。缓存不是本轮 request 放大的原因。

## 6. Simple Trace

TaskSpace 的 11 个请求主要路径如下：

| Request | 动作 | 结果 |
|---:|---|---|
| 1 | `exec_command` | 空 Map hard gate 拒绝 |
| 2 | `initialize_map(required_next_call=ordinary_tool)` | 缺 sibling，整响应零执行 |
| 3 | `initialize_map + exec_command` | 同响应成功 |
| 4-6 | 并行读取、代码读取、失败测试 | 正常工作 |
| 7 | `complete_then_continue(required_next_call=apply_patch)` | 缺 sibling，整响应零执行 |
| 8 | `complete_then_continue + apply_patch` | 同响应成功，patch 原生反馈 |
| 9 | pytest | 通过 |
| 10 | `complete_then_continue + pytest` | 同响应成功 |
| 11 | `complete_then_end` | Map 闭合 |

两次 preflight 反馈都完整进入上下文；Agent 下一请求明确说明需要 sibling，并正确纠正。没有反馈丢失、
歧义或 Runtime 语义改写。

## 7. Complex Trace

复杂样本的 25 次请求不应全部归因于 `required_next_call`：

1. 初始化 control 单独返回一次，下一请求改为 `initialize_map + find`。
2. Agent 错误地对已在运行的 `inspect` 再次 `bind`；第一次缺 patch sibling，第二次与 patch 合并但状态失败，
   patch 被机械跳过。该问题是 Agent 对 Map 状态的错误理解，不是 feedback 缺失。
3. 正确的 `complete_then_continue + apply_patch` 保持同响应；patch 因上下文匹配失败而返回原生错误。
4. Agent 后续分多次读取和修补，其中一次引入重复 `pro` 键后自行删除。
5. Agent 在一个 response 中生成 3 个 `apply_patch`，request-wide preflight 零执行拒绝；下一请求改为一个
   多文件 patch 并成功。单 response 单 patch 硬规则按设计生效。
6. Agent 过早执行 `complete_then_end`，因 Finish 未 Ready 被拒绝；随后用
   `complete_then_continue + pytest`，最终 `complete_then_end` 闭合。

因此 complex 的 14 次 request 差额由多个独立行为叠加：2 次 sibling 遗漏、错误 bind 及恢复、patch
上下文失败及重读、拆分 patch、自行修正重复键、3-patch preflight、过早 terminal。单样本不能证明这些
全部由字段改名引入。

## 8. 当前判断

`required_next_call` 比旧 `continuation` 更忠实地表达“声明而非执行”，应保留；回退会重新引入已确认的
语义歧义。但一个 function tool 的 JSON Schema 只能约束自身 arguments，不能从结构上保证 provider 继续
生成第二个顶层 tool call。当前 hard preflight 可以保证正确性，却不能保证零重试。

下一步若继续优化首次采用率，必须单独设计和验证 provider-visible 调用协议；不能让 Runtime 自动补 sibling，
也不能把 patch 重新塞回 control。R7 Phase E 在该效率问题收敛前保持未启动。
