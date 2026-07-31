# 真实缓存回归的授权预算边界

- 日期：2026-08-01
- 状态：离线实现与测试通过；未运行真实 provider
- 适用范围：`run_cache_hit_regression.py` 启动的获批缓存 smoke

## 1. 产品边界

预算单把成本信息分成两类，不能混称为“上限”：

1. **执行前和执行中的硬边界**：精确源码、sample/arm/repeat 矩阵、禁止自动重试、每个 Whale 进程最多发出的
   provider 请求数、agent 容器运行时限，以及超时后的容器清理宽限。
2. **执行后的观测阈值**：常态 input/output token 和按实际 usage 估算的费用。超过阈值会停止后续 sample 并禁止
   晋升，但已经完成的 provider 请求不能被事后撤销，因此这些值不得宣称为硬成本边界。

预算单另给出保守最坏费用：

```text
最大 sample run × 每 run 请求硬上限 × provider 官方单请求最大 input/output × 冻结价格
```

input 全部按 cache miss 价格计算。该值明显高于常态观测预算，但它诚实表达当前 provider 基础设施下可证明的最坏
边界；不得用历史均值替换它。

## 2. 执行机制

- 通用 provider client 从 `WHALE_PROVIDER_REQUEST_HARD_LIMIT` 读取正整数；缺失表示普通产品运行不启用专项限制，
  非法值 fail closed。
- 计数器是进程共享的，主 Agent、子 Agent、HTTP/WS retry、压缩、memory 和 realtime 请求共同消耗同一额度。
- 每次真实 dispatch 前原子认领额度，达到上限时在网络请求前返回明确错误。
- runner 只把已授权值传给 Whale 子进程，不改变 TaskSpace、Standard、普通 Tool 或 Agent 决策语义。
- 外层 runner 超时后按唯一 `whalecode.run_id` Docker 标签枚举并强制清理残留容器；清理结果写入 attempt。
- 失败运行只有部分 usage 时，账本记录 `estimated_partial` 和“已知最低值”；完全没有遥测时记录 `unavailable`，
  不写成零成本或完整结算。

## 3. 失败关闭

- 授权与预算摘要不一致、请求上限缺失、官方 provider 上限缺失、价格快照缺失或 proposal 被修改：启动前拒绝。
- 任一 attempt 失败、usage 不完整、观测阈值越界或超时清理失败：结果不得晋升。
- 进程崩溃导致结果已写但账本未结算：恢复命令按持久 result 幂等补结算；没有 result 时保留 `running/unsettled`，
  不猜测费用。

## 4. 已验证证据

- Python cache control plane：110 项通过，包括预算复算、篡改拒绝、授权重放、超时回收、部分费用和恢复。
- Rust：3 项 provider hard-limit 单测通过，包括超额前置拒绝、非法配置 fail closed 和多 client 共享计数。
- PowerShell：runner 语法、E3 start gate、release decision、non-agent builder 自测通过。
- 本轮真实 Whale Agent/provider run：0；全局付费运行账本没有新增记录。

## 5. 外部依据

- [DeepSeek 模型与价格](https://api-docs.deepseek.com/quick_start/pricing/)：冻结 Flash 的上下文、最大输出和
  cached/uncached/output 价格，用于预算最坏值。
- [DeepSeek Rate Limit](https://api-docs.deepseek.com/quick_start/rate_limit)：provider 只说明账号并发调度，
  不提供单次作业费用上限，因此项目必须在自身 provider dispatch 层限制请求数。
