# R7 v1.0.4 重复运行与尾部异常分析

## 1. 结论

本轮在同一 Docker 基线、`deepseek-v4-flash`、`map-request` 与当前 R7 v1.0.4
实现上，对 simple/complex 各执行 3 组 Standard/TaskSpace paired repeats。12 个 side
全部完成，公开与隐藏验证全部通过，performance observation 没有 incomplete、skipped 或 warning。

先前 complex TaskSpace 的 25 requests 没有在新增三次中复现；新增结果为
`15/14/15`，中位数 15。它不是确定性的 Runtime 回归，而是一次 Patch 失败后出现了异常长的
Agent 恢复路径。但这条 25-request bad case 仍然是有效的尾部成本证据，不能从统计中删除。

重复运行同时确认了三类稳定问题：

1. `required_next_call` 首次采用率不足：6 次 TaskSpace 中共声明 29 次，满足 19 次，遗漏
   10 次，遗漏率 34.48%，每次运行至少发生 1 次。
2. Agent 经常先完成实际工作，再被迫补走 Map 节点；补走动作包括 `echo` 和重复 pytest，
   表明 Map 在部分运行中退化为事后记账。
3. TaskSpace 的缓存链路健康，但额外请求与每请求固定上下文共同令 input 成本稳定高于
   Standard；缓存命中不能消除这部分总输入放大。

## 2. 运行范围与有效性

| 项目 | 值 |
|---|---|
| Simple | `single-file-fast-fix`，3 pairs |
| Complex | `subscription-billing-repair`，3 pairs |
| 每个 pair | Standard 1 次 + TaskSpace 1 次 |
| 模型 | `deepseek-v4-flash` |
| Projection policy | `map-request` |
| 执行环境 | Docker，统一 Whale debug binary |
| 结果 | 12/12 side solved，公开/隐藏验证均通过 |
| Observation | 12/12 complete，0 skipped，0 incomplete，0 warning |

首次启动时两个 suite 都因当前 shell 未加载 `.env.local` 而在 provider credential
preflight 中止；没有发出 provider 请求。随后通过 shell 环境加载本地 `.env.local` 后重跑。
无效 harness 运行不计入下表，也未把密钥写入命令行或日志。

## 3. 三次聚合结果

### 3.1 Simple

| Mode | Solved | Requests 总/均值/中位数 | Wall(s) 总/均值/中位数 | Input 总/均值/中位数 | Req2+ cache |
|---|---:|---:|---:|---:|---:|
| Standard | 3/3 | 25 / 8.33 / 8 | 63.63 / 21.21 / 20.68 | 185,449 / 61,816 / 61,656 | 96.48% |
| TaskSpace | 3/3 | 32 / 10.67 / 11 | 98.21 / 32.74 / 30.60 | 333,510 / 111,170 / 115,602 | 96.47% |

TaskSpace / Standard：requests 1.28x、wall 1.54x、input 1.80x、uncached input
1.79x、output 1.67x。TaskSpace runtime tools 反而是 0.94x，说明放大主要来自 provider
轮次和上下文，而不是做了更多有效工具工作。

### 3.2 Complex

| Mode | Solved | Requests 总/均值/中位数 | Wall(s) 总/均值/中位数 | Input 总/均值/中位数 | Req2+ cache |
|---|---:|---:|---:|---:|---:|
| Standard | 3/3 | 36 / 12.00 / 14 | 143.93 / 47.98 / 47.24 | 387,951 / 129,317 / 151,638 | 95.59% |
| TaskSpace | 3/3 | 44 / 14.67 / 15 | 184.16 / 61.39 / 61.38 | 656,569 / 218,856 / 223,662 | 96.26% |

TaskSpace / Standard：requests 1.22x、wall 1.28x、input 1.69x、uncached input
1.44x、output 1.33x。TaskSpace cache 比 Standard 高 0.67 个百分点，缓存不是请求或
input 放大的原因。

## 4. 25 Requests Bad Case 的定位

紧邻本轮之前的同版本 complex 结果是 Standard 11、TaskSpace 25 requests。新增三次的
TaskSpace 是 `15/14/15`。四次可比观察合并后：

| Mode | Requests | 总和 | 均值 | 中位数 |
|---|---|---:|---:|---:|
| Standard | `11/14/8/14` | 47 | 11.75 | 12.50 |
| TaskSpace | `25/15/14/15` | 69 | 17.25 | 15.00 |

25-request 运行比新增 TaskSpace 中位数多 10 次。新增三次 TaskSpace 都发生了 1 次
Patch prepare failure，却分别只使用 3、2、2 个 Patch 声明并在 14-15 requests 内结束。
这证明“发生 Patch 失败”不是 25 requests 的充分原因。

异常运行的决定性差异是恢复质量：它在第一次 Patch 失败后经历了重复读文件、先添加重复
`pro` 键再删除、连续检查、同响应生成 3 个 `apply_patch` 被 preflight 拒绝，以及过早
`complete_then_end`。该恢复段比正常重复运行多约 10 个请求。

Patch 不稳定也不是 TaskSpace 独有：新增 complex 的 Standard 三次中有两次出现 Patch
prepare failure，共 3 次失败；TaskSpace 三次各 1 次。simple 中 Standard 也有两次运行出现
Patch prepare failure，而 TaskSpace 三次均一次成功。当前证据不支持把 Patch 格式错误归因于
TaskSpace feedback 或 projection。

## 5. 稳定复现的问题

### 5.1 组合调用首次遗漏

6 次 TaskSpace 共 29 个 `required_next_call` 声明，19 次满足、10 次遗漏：

- 8 次发生在 `initialize_map`，Agent 单独返回 control 后才在下一请求补 ordinary sibling。
- 2 次发生在 simple 的 Patch handoff，下一请求才补 `apply_patch` sibling。
- 6/6 运行至少发生一次遗漏。

因此 production probe 的 6/6 sibling 成功只证明 provider 能力存在；自然 coding 流程中的
首次采用率仍未通过。当前 hard preflight 保证了正确性，但把每次遗漏转换为一个确定的额外请求。

### 5.2 初始化前普通动作

6 次中有 2 次先调用普通工具再初始化 Map：一次 simple 调用 `find`，一次 complex 在同一响应
调用 `find` 和 `cat README`。两次都收到忠实的 `no_task_path`，随后正确初始化。反馈没有丢失，
但工作协议尚未稳定进入 Agent 的首次行动选择。

### 5.3 Map 事后补账

多次运行中，Agent 在初始探索节点内直接完成读取、Patch 和测试，最后才发现后续 `fix`、
`verify` 等节点尚未闭合。为了满足硬状态，它随后使用：

- `echo "...already complete"` 作为 required ordinary action；
- 重复执行已经通过的 pytest；
- 先错误 `complete_then_end`，再回头逐个完成节点。

这些动作符合机械 schema，却没有新增工程证据。问题不应通过 Runtime 理解 `echo` 或判断工作语义来
解决；它说明当前工具协议没有让 Agent 在真实动作发生时自然同步 Map，最终硬闭合把偏差转成了
事后流程成本。

## 6. 成本解释

三次聚合中，TaskSpace requests 只比 Standard 多 22%-28%，但 input 多 69%-80%。除更多
请求反复携带历史外，TaskSpace 每个请求还稳定多出约 1.9K tokens 的 system/tool schema 固定面：

- system messages 约 `755 vs 180 tokens/request`；
- tools 约 `6,750 vs 5,418 tokens/request`。

Req2+ cache 为 95.59%-96.48%，provider prefix 保存为 100%。所以这是高命中缓存下的总输入
放大，不是缓存断裂；商业成本仍需冻结 provider 单价后才能计算。

## 7. 工程判断

1. v1.0.4 没有产生“复杂样本必然退化到 25 requests”的确定性回归。
2. 25-request 运行是有效尾部异常，暴露了失败恢复路径缺乏稳定性；3 次未复现不足以证明尾部风险消失。
3. 当前更确定的问题不是 Patch feedback，而是首次协议采用率和 Map 与实际动作不同步。
4. 后续优化应聚焦 tool schema 与工作协议的自然配合，减少遗漏和事后补账；不能让 Runtime 解析
   `echo`、推断任务语义或自动替 Agent 迁移节点。

## 8. 证据路径

```text
target/r7-required-next-call-v104-repeat-analysis/simple/single-file-fast-fix/20260719-230858-711
target/r7-required-next-call-v104-repeat-analysis/complex/subscription-billing-repair/20260719-230908-711
target/r7-required-next-call-v104-validation/complex/subscription-billing-repair/20260719-214822-447
```
