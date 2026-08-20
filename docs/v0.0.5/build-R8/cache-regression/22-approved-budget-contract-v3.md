# 缓存回归批准预算合同 v3

## 问题

旧 v2 提案把 `请求数 × Provider 单请求最大上下文/输出` 记为 `maximums`，授权合同又要求用户原样批准该字段。人工选择的 input/output 上限只是 sample 结束后的观测阈值。因此一次批准 40 万 input、2 万 output 的运行，会被授权文件放大成 1200 万 input、460.8 万 output。

## 收敛后的合同

- 新提案使用 `whalecode-cache-budget-proposal-v3`，新授权使用 `whalecode-cache-budget-authorization-v2`。
- `approved_maximums` 只包含人工选定矩阵对应的请求、input、output、耗时和费用停止额度。
- 费用由批准的 token 上限和冻结价格按全部 input 未命中的最坏情况机械推导，不接受第二个可冲突的费用输入。
- `provider_capacity_ceiling` 单独披露 Provider 的理论容量，只用于风险说明，不再成为用户授权对象。
- 历史 v2/v1 提案与授权只读验证，不能用于创建新运行。

## 执行边界

隔离 Provider boundary 继续在请求发送前执行请求数硬限制，并新增 terminal usage 累计：input、cached input、output 和冻结价格估算费用。usage 缺失或任一批准额度达到后，下一次 Provider 请求在发送前被拒绝。已经在途的单个请求不能被返回后的 usage 反向截断；该边界在提案中显式记录，不再把事后阈值描述成 Provider 单请求硬上限。

本次目标运行的 40 万 input、2 万 output 按 DeepSeek V4 Flash 当前中文价格上界推导为 0.44 元，低于用户批准的 1.40 元总包。

## 验证

- cache regression Python：229 passed。
- Provider boundary Python：9 passed。
- Docker provider boundary：passed。
- container benchmark runner：passed。
- 新合同测试覆盖授权额度与 Provider 容量分离、usage 缺失 fail closed、累计额度达到后拒绝下一请求、SSE terminal usage 分块解析。

真实 Provider 验收不属于本文件的免费验证；使用已批准的单样本预算另行登记并结算。
