# affected node 状态反馈五轮结果

- Date: 2026-08-17
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arm: `map-request`
- Repeat: 5 个独立 `repeat=1`，并行执行
- Subject commit: `a36a42939`
- Candidate binary: `3ee814555d86079b67398690ccb146ad3c8dc75ec2f3899eb7bb1800b4ce66b3`
- Ledger: `WAR-20260817-020757-AFFECTED-STATE-R5`

## 1. 单变量

本轮只补充成功 `taskspace_exec` 的事实反馈：返回本批次直接操作或机械变更节点的前后状态，并在仍未完成的 owner 上列出
当前不可执行的直接 Work 子节点及其未完成父节点。状态机、合法序列、Tool 执行、Base instructions 和拒绝规则均未改变。

这与已回退的 `owner_state_after` 实验不同：旧实验只回传 client owner 的一个 post-state；本轮返回本批次相关 canonical
状态及精确依赖事实。聚焦测试为 69 passed，缓存敏感面门禁通过。

## 2. 逐轮结果

| Run | 业务/Map | Requests | Input | Cached | Uncached | Output | Agent wall | Frontier early | 其他异常 |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | passed | 9 | 141,583 | 121,984 | 19,599 | 3,200 | 28.778s | 0 | REDUNDANT-INFLIGHT；Waiting verify 直接 completed 被拒 |
| 2 | passed | 7 | 105,518 | 90,240 | 15,278 | 1,926 | 18.323s | 0 | none |
| 3 | passed | 12 | 185,627 | 177,152 | 8,475 | 2,599 | 27.109s | 1 | 4 次 JSON syntax；1 次 arguments wrapper |
| 4 | passed | 8 | 125,511 | 102,656 | 22,855 | 3,017 | 27.245s | 1 | REDUNDANT-INFLIGHT |
| 5 | passed | 8 | 118,353 | 104,320 | 14,033 | 1,934 | 20.780s | 0 | REDUNDANT-INFLIGHT |
| **Total** | **5/5** | **44** | **676,592** | **596,352** | **80,240** | **12,676** | **122.235s** | **2/5** | **11 rejects / 4 runs** |
| **Mean** | - | **8.8** | **135,318.4** | **119,270.4** | **16,048** | **2,535.2** | **24.447s** | - | - |
| **Median** | - | **8** | **125,511** | **104,320** | **15,278** | **2,599** | **27.109s** | - | - |

五轮全量缓存命中率为 `88.14%`；Request 2+ 加权缓存命中率为 `91.79%`。按冻结价格估算费用为
`0.11751904 CNY`。五个 wrapper 因 right-only 跳过 Standard 而返回非零，但目标侧均为 Agent complete、外部验证通过、隐藏
oracle 通过且 Map 闭合，不是业务失败。

## 3. 反馈正确性

实际 Tool output 逐轮包含：

```json
{
  "node_id": "fix",
  "state_before_sequence": "waiting",
  "state_after_sequence": "in_flight",
  "unavailable_direct_work_children": [
    {
      "node_id": "verify",
      "state": "waiting",
      "incomplete_parent_ids": ["fix"]
    }
  ]
}
```

它与持久化 Map 一致，只陈述直接子节点和未完成父节点，没有替 Agent 选择动作，也没有改变节点状态。Run 2 正确使用
`update_and_work(fix=completed) + exec_command@verify`；Run 1 将测试留在 `fix`，随后尝试把仍为 Waiting 的 `verify`
直接完成；Run 3/4 则在看到同一事实后仍先提交 `work@verify`。这证明反馈进入了上下文，但 Agent 使用仍不稳定。

## 4. 行为与成本判断

当前十轮基线的 `FRONTIER-EARLY` 为 `4/10`，本轮为 `2/5`，比例相同。相对该基线均值，本轮 requests `+17.33%`、
input `+22.41%`、uncached input `+106.70%`、output `+15.67%`、Agent wall `+1.13%`。其中 Run 3 的五次参数合同错误和
其余状态拒绝显著放大请求与输入；五轮样本不足以把全部增量归因于反馈字段本身，但也没有观察到可接受的行为或成本收益。

因此结论分为两层：

1. `taskspace_exec` 反馈此前缺少本批次 canonical 节点状态，当前实现正确补全了产品语义。
2. 该信息不是 Waiting frontier 误选的充分修复；I04 仍是合法序列分支选择问题，不能继续通过堆叠同义反馈解决。

当前候选保留为反馈正确性修复，但不据此关闭 I04，也不晋升缓存基线。下一步是否继续保留需要同时权衡产品语义收益和固定
feedback 成本，不应把 5/5 业务成功误报为 Waiting 问题已解决。

## 5. 证据

- `target/r8-feedback-candidate/repeat5-{1..5}/single-file-fast-fix/*`
- 每个 root 均包含 `request-summary.json`、`provider-cache-trace-summary.json`、`rollout.jsonl`、验证和 Map 证据。
