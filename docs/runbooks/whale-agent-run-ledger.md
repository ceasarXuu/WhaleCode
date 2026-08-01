# Whale Agent 运行流水账

## 目的

所有真实 Whale Agent 运行统一登记在：

`benchmarks/whale-agent-run-ledger.json`

该文件是跨版本的全局成本事实源。TaskSpace、Standard、benchmark、provider probe、
smoke、重试和人工验证均使用同一账本，不在版本目录下另建平行账本。

## 记账单位

一次命令、脚本或矩阵启动对应一个 `run_batch`。其中真实执行规模按
`sample × arm × repeat` 计算；重试是新的批次，必须新建 `record_id`，不能覆盖旧记录。

历史上无法逐批还原的运行可以使用 `historical_aggregate`，但必须说明聚合范围、证据和
缺失事实。

## 运行前

1. 在账本中创建 `status=planned` 的记录。
2. 写明运行理由、模型、sample、arm、repeat、计划执行数、预计 API 请求、token/费用预算、
   最长耗时和停止条件。
3. 计划执行数超过 3 时，先向用户申请专项预算，并将批准范围写入
   `authorization.reference` 和 `authorization.budget_summary`。
4. 没有计划记录，或大规模运行没有 `authorization.status=granted`，不得启动真实 Agent。

API Key 可用只表示具备技术条件，不表示获得费用授权。

## 运行后

无论成功、失败、超时、取消或零请求退出，都立即结算同一记录：

- 起止时间、自然耗时和 Agent wall time；
- 实际 sample run 和 API request 数；
- input、cached input、uncached input、output token；
- 实际账单金额，或带价格快照的估算金额；
- 最终状态、停止原因和 run manifest/metrics/trace 路径。

`input` 必须等于 `cached_input + uncached_input`。不能从 rollout 文件大小或文本长度推测
provider token。

## 费用口径

费用来源优先级：

1. Provider 账户或响应提供的实际扣费；
2. 运行时冻结的官方模型价格快照；
3. 两者均不可得时，明确记录 `status=unavailable` 和原因，不能留空或猜测。

DeepSeek 官方价格以百万 token 为单位，缓存命中输入、缓存未命中输入和输出分别计价：

`cached_input / 1e6 × cached_price + uncached_input / 1e6 × uncached_price + output / 1e6 × output_price`

价格来源：[DeepSeek 模型与价格](https://api-docs.deepseek.com/zh-cn/quick_start/pricing/)。
价格会变化，每条估算记录必须保存自己的抓取时间和单价，不能使用当前价格静默改写历史费用。

## 校验

只执行本地账本校验，不会启动 Whale Agent 或访问 provider：

```bash
pwsh -NoProfile -File scripts/taskspace-benchmark/test-whale-agent-run-ledger.ps1
```

账本结构由 `benchmarks/whale-agent-run-ledger-v1.schema.json` 描述。修改账本、价格字段或
授权规则后必须运行该校验。
