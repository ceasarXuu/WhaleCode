# 01. Evidence and Root-Cause Input

## 1. v0.0.4 关键事实

v0.0.4 E3 的 clean run 已经足够支撑工程诊断：

```text
Standard solved: 7/15
TaskSpace solved: 8/15
TaskSpace net gain: +1 pair
TaskSpace agent time: 4.99x Standard
TaskSpace direct input+output tokens: 19.92x Standard
Tool calls: only 1.20x Standard
high_unreviewed_result_ratio: 15/15 TaskSpace runs
high_blocked_node_ratio: 13/15 TaskSpace runs
subagent_no_decision_yield: 7/15 TaskSpace runs
```

这说明 TaskSpace v0.0.4 的工程可审计性明显改善，但产品收益和成本收益比没有成立。

## 2. 根因已经基本闭合

v0.0.4 的 token/time 膨胀不是 map 摘要本身过大，也不是 usage accounting 简单 bug。更准确的根因是：

```text
TaskSpace input token bloat
≈ model request count ratio × avg input/request ratio
≈ 9.31x × 2.16x
≈ 20.11x
```

其中：

```text
Standard model-request proxy count: 132
TaskSpace model-request proxy count: 1,229
Standard avg input/request: ~19,126 tokens
TaskSpace avg input/request: ~41,318 tokens
```

耗时也主要是 request count 驱动：

```text
Standard avg walltime/request: ~6.00s
TaskSpace avg walltime/request: ~3.22s
```

因此，v0.0.5 第一优先级不是“每轮上下文再压一点”，而是减少模型可见协议轮次。

## 3. 一阶根因：模型可见的细粒度控制协议

v0.0.4 的 TaskSpace 把问题状态管理拆成大量模型可见工具动作：

```text
finish_node = 209
mark_result_validity = 149
record_success_criteria = 114
bind_node = 61
block_node = 60
create_node = 54
record_decision = 31
adopt_result = 22
```

15 个 TaskSpace run 合计：

```text
taskspace_control calls = 850
spawn_agent calls = 68
wait_agent calls = 12
snapshot_updated events = 812
```

这些动作并不是没有价值，但它们的粒度太细，并且每一步都需要模型参与、工具调用、历史重放和下一轮推理。

### 工程解释

当前实现接近：

```text
模型读 TaskSpace protocol / context
模型调用 taskspace_control(action A)
runtime 更新状态
模型再次读上下文
模型调用 taskspace_control(action B)
...
```

v0.0.5 要改成：

```text
模型完成一个阶段推理
模型一次 state_commit 提交状态变化
runtime 批量更新 ledger/node/result/decision
runtime 自动计算 next-valid-action / projection
模型继续处理真正业务步骤
```

## 4. 二阶根因：每轮上下文变大

TaskSpace 每次请求平均 input 是 Standard 的约 2.16x。原因不是单个 map 大到不可接受，而是多类上下文叠加：

```text
standard conversation/tool history
+ TaskSpace developer protocol
+ task inventory / active task path
+ node list / current node contract
+ problem ledger
+ result summary
+ blocked/unreviewed graph health context
+ subagent summaries
+ function call / function output history
```

map/ledger 没有真正替代旧 history，而是叠加在 history 上。

## 5. 局部放大器：大工具输出重放

部分 outlier 由大工具输出进入历史后反复重放造成。例如 `analyze-access-logs pair-005`：

```text
Get-Content full access_log output: ~169KB
进入前 model input: ~16.5k tokens
进入后下一次 model input: ~105k tokens
后续请求持续 >105k tokens
```

这不是全局主因，但会与 TaskSpace 的高轮次控制循环相乘，形成局部 90x+ token outlier。

## 6. 质量根因：map 记录多，采纳少

TaskSpace v0.0.4 已经把 result/adoption/graph health 问题暴露出来，但没有解决：

```text
high_unreviewed_result_ratio: 15/15
subagent_no_decision_yield: 7/15
```

这说明 map 产生中间信息的速度超过了采纳、压缩和废弃能力。没有被采纳的 result、blocked node、stale branch 会变成 context debt。

## 7. v0.0.5 设计约束

由以上根因得到设计约束：

| 约束 | 设计响应 |
|---|---|
| 请求轮数是最大乘数 | 批量 state_commit、auto bookkeeping、next-valid-action gate |
| 每请求上下文是第二乘数 | context projection、static/dynamic split、history elision |
| 大输出会形成 outlier | output referenceization、slice-on-demand、hard output cap |
| unreviewed result 形成债务 | result lifecycle、batch adoption、GC/archive |
| map 尚未替代 history | shadow compaction、semantic projection、history replacement metrics |
| 简单任务被重型化 | thin routing、verification-first path、escalation-only TaskSpace |

## 8. 设计结论

v0.0.5 的正确工程方向是：

```text
把 TaskSpace 从模型频繁操作的显式协议，
改成 runtime 托管的紧凑状态机和语义投影系统。
```
