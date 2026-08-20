# owner state 反馈单变量五轮结果

- Date: 2026-08-16
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arm: `map-request`
- Repeat: 5 个独立 `repeat=1`，并行执行
- Candidate commit: `96254de81`
- Candidate binary: `5cf9d12d6622312dc0abb778fae2e3b0f10486c3adaa06ec49b35dd22cd5f6ae`
- Ledger: `WAR-20260816-235021-OWNER-STATE-R5`

## 1. 单变量

候选只在每条 client Tool 结果中增加必填 `owner_state_after`，机械复制本批次已持久化 canonical Map 中的 owner state。
序列 schema、Base instructions、DAG、拒绝和 Tool 执行逻辑均未改动。目标是验证成功 Tool feedback 缺少 owner state 是否是
`patch -> verify` Waiting frontier 误选的主要诱因。

聚焦 TaskSpace Exec 测试为 67 passed，缓存敏感面门禁通过。真实运行后因目标收益未成立，候选已由 `52d209637` 回退。

## 2. 逐轮结果

| Run | 结果 | Requests | Input | Cached | Uncached | Output | Agent wall | Waiting frontier | 其他异常 |
|---:|---|---:|---:|---:|---:|---:|---:|---|---|
| 1 | solved | 8 | 118,960 | 110,592 | 8,368 | 2,125 | 20.280s | 1 | none |
| 2 | solved | 6 | 85,127 | 78,592 | 6,535 | 1,691 | 15.140s | 0 | none |
| 3 | solved | 8 | 120,491 | 111,872 | 8,619 | 2,246 | 19.840s | 1 | none |
| 4 | solved | 6 | 86,117 | 81,920 | 4,197 | 1,857 | 14.913s | 0 | none |
| 5 | incomplete | 2 | 25,528 | 24,448 | 1,080 | 450 | 4.866s | 未到达 | 顶层 `exec_command` 逃逸 |
| **Total** | **4/5 solved** | **30** | **436,223** | **407,424** | **28,799** | **8,369** | **75.039s** | **2 events** | **1 fatal** |

五轮全量缓存命中率为 `407,424 / 436,223 = 93.40%`。本轮无 Standard 臂，不形成相对成本结论。按冻结价格估算费用为
`0.05368548 CNY`。

## 3. 因果判断

Run 1 和 Run 3 的 `apply_patch@fix` 成功结果均逐字包含：

```json
{"node_id":"fix","owner_state_after":"in_flight","tool":"apply_patch","outcome":"succeeded"}
```

下一请求的 reasoning 仍只提取“patch 已应用，现在运行测试”，并生成 `work(exec_command@verify)`。Runtime 随后准确拒绝：
`verify=waiting`，未完成直接父节点为 `fix`。两轮都在再下一请求改为
`update_and_work(fix=completed) + exec_command@verify`。

因此：

1. owner state 已忠实进入 Agent 上下文，排除字段没有实际生效。
2. 四轮实际到达目标边界，误选为 `2/4`；当前十轮基线为 `4/10`，未观察到下降。
3. 该结果直接证明 owner state 省略不是充分根因，也不支持把补字段晋升为当前修复。
4. 样本量不足以排除它存在很小的概率贡献，但继续保留字段会扩展 output schema 和每次 client feedback，收益证据不足。

## 4. 独立异常

Run 5 首请求正确执行 `initialize_and_work + exec_command@inspect`，第二请求却生成与 `taskspace_exec` 同级的未声明顶层
`exec_command`。Runtime 的 response reconciliation 在执行前以
`TaskSpace response contains forbidden top-level client Tool` 终止，文件没有被修改。该异常属于 I03 的顶层 client Tool 逃逸，
与 Waiting frontier 候选不是同一问题，不能计作候选成功或失败的目标分母。

## 5. 结论

候选不通过。Waiting frontier 的剩余主方向回到序列分支选择：Agent 即使已经知道 owner 为 `in_flight`，仍可能让自然的
“patch 后测试”动作压过显式 lifecycle handoff。下一项实验应只改变合法序列分支的结构显著性，不继续给反馈堆叠同义状态信息，
也不让 Runtime 自动完成节点。

## 6. 证据

- `target/r8-owner-state-feedback/repeat5-{1..5}/single-file-fast-fix/*`
- 每个 root 均有 `performance-observation.{json,md}`、`request-facts.json`、`provider-wire-trace.jsonl` 和 `rollout.jsonl`。
