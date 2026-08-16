# IC-09 机械反馈收敛 Repeat 5 结果

- Status: complete / behavior stable / remaining cost open
- Date: 2026-08-17
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arms: Standard + `map-request`
- Repeat: 5 pairs / 10 sample runs
- Subject commit: `5fdf19cdd`
- Ledger: `WAR-20260817-061913-R8-I08-IC09-R5`
- Evidence: `target/r8-i08/ic09-feedback-compaction-repeat5/single-file-fast-fix/20260817-062002-587`

## 1. 结果

| 指标 | Standard 总和 | Standard 均值 / 中位数 | TaskSpace 总和 | TaskSpace 均值 / 中位数 | TS / Standard |
|---|---:|---:|---:|---:|---:|
| 业务、公开验证、隐藏 oracle | 5/5 | - | 5/5 | - | 等价 |
| Requests | 31 | 6.2 / 6 | 35 | 7.0 / 7 | 1.13x |
| Input | 381,252 | 76,250 / 75,101 | 531,515 | 106,303 / 106,915 | 1.39x |
| Cached input | 373,504 | 74,701 / 73,216 | 497,664 | 99,533 / 100,096 | 1.33x |
| Uncached input | 7,748 | 1,550 / 1,580 | 33,851 | 6,770 / 6,819 | 4.37x |
| Output | 6,037 | 1,207 / 1,094 | 12,283 | 2,457 / 2,420 | 2.03x |
| Agent wall | 63.69s | 12.74s / 12.21s | 106.06s | 21.21s / 20.56s | 1.67x |
| Request 2+ cache | 97.68% | weighted | 92.86% | weighted | -4.82pp |

总计 66 requests、912,767 input、18,320 output，冻结价格下估算 CNY 0.09566236，远低于 CNY 3.00 上限。
runner shell exit 1 来自 `aggregate_not_enabled`；正式 `run-status` 为 `valid`、exit code 0，10 个 side 均完成。

## 2. 反馈收敛效果

当前五轮共有 27 个成功 Exec output，合计 29,105 B，平均 1,078 B/output。变更前最近同 sample 的五轮有 29 个成功 output，
合计 43,209 B，平均 1,490 B/output。平均载体缩小 27.65%，与静态反算的 30.25% 同方向；成功结果、原生 Tool output、
节点归属和状态反馈均未丢失。

相对变更前 TaskSpace-only 五轮，当前 requests 为 35 vs 34，input 为 531,515 vs 528,450；路径波动使总 input 增加 0.58%，
但平均每请求 input 从 15,543 降至 15,186，下降 2.29%。因此可以确认机械反馈缩小，但不能声称它单独降低了整批总 input。

按相邻请求的 input 增量观察，当前 TaskSpace 30 个增量合计 20,634，均值 687.80、中位数 784；变更前 TaskSpace 29 个增量
合计 24,583，均值 847.69、中位数 883。均值下降 18.86%，中位数下降 11.21%，累计下降 16.06%。同轮 Standard 的均值为
436.42，因此当前 TaskSpace 单轮 input 增量仍为 Standard 的 1.58x。后续成本优化以这个相邻请求增量为主指标，不再用总 input
掩盖单轮载体变化。

## 3. 行为与异常

- 五轮 Map 均为 `root -> inspect -> fix -> verify -> finish`，5 nodes / 4 edges，最终全部 completed、0 open leaves；无显式
  `read_map`。
- 3/5 TaskSpace runs 无协议或状态拒绝。
- Run 1 和 Run 5 各有一次 `TransitionInvalid`：Agent 在同一 `update_and_work` 中完成 `inspect`，又显式把仍为 waiting 的
  `fix` 写成 `in_flight`。Runtime 在任何 Map/Tool 副作用前拒绝；Agent 下一请求删除多余状态声明后完成 Patch。
- Run 5 另有一次未转义换行造成的 JSON syntax reject；零副作用，下一请求纠正。现有单闭合符自愈不覆盖字符串控制字符。
- 没有 Waiting child work 被执行、普通 Tool 提交失败、状态反馈解析失败或最终结果失败。Standard Run 5 的 failed Tool 是修复前
  `pytest` 返回 1，属于正常诊断结果。

这些异常不是由删除固定反馈字段直接诱发：Agent 已拿到 `inspect=in_flight`、`fix=waiting` 和未完成父节点说明；错误发生在它又
显式提交派生 `in_flight` 状态。它们分别继续归入 I04 的状态操作合同和 I03 的参数构造稳定性，不回滚 IC-09。

## 4. 结论

IC-09 的机械反馈收敛通过 repeat=5：正确性和 Map 完整性无回归，反馈面积稳定缩小。R8-I08 仍保持 open，因为 TaskSpace 固定
Tool wire 为 26,688 B/request，且请求、自然历史和未缓存 input 仍高于 Standard。本轮不继续添加 Runtime 语义约束，也不把
两类 Agent 错误混入反馈压缩结论。

## 5. 两项异常的离线修复

2026-08-17 在不启动新真实运行的前提下完成两项最小修复：

1. 自愈器新增“JSON 字符串内唯一一个裸 LF 转义为 `\\n`”候选。仍要求修复后通过当前 TaskSpace Catalog 完整解码，且全局
   只有一个合法候选；多个裸换行、复合语法错误或语义错误仍原样拒绝。修复发生在正式历史落账前，错误参数不会进入后续上下文。
2. TaskSpace Base 升级为 `3.0.5`，明确父节点满足目标后应先显式完成，Runtime 再机械派生依赖节点 Ready；Agent 可以在同一
   响应提交刚解锁子节点的 Tool 动作，但不得显式把 waiting 子节点改成 `in_flight`。Tool schema、状态机和拒绝规则未改变。

相关 `taskspace_exec` 测试 74 项、Base 合同测试 3 项和会话历史替换测试均通过。两项当前仅为离线修复完成，是否降低真实
syntax/frontier 异常频率仍待后续获批运行验证；不得据此关闭 I03/I04。
