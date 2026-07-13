# 自然 Active Map 延续样本

该样本用于观察 Map projection 压缩，不用于证明某个固定答案。前缀来自 2026-07-14 已完成的一次
`subscription-billing-repair` 正常运行，早于本 fixture 的建设；不是由测试代码伪造的 TaskSpace 事件。

## 截断规则

选择源 rollout 中第一个已提交的 `taskspace_control` 结果，且同时满足：

- Task 和 Map 均为 `active`；
- 恰有 3 个 completed 节点；
- `run_tests` 是唯一 open/current 节点；
- 下一步测试尚未执行。

该事件位于源 rollout 第 324 行、task context sequence 68。压缩包解压后必须保持逐字节 SHA-256，runner
不得删除、重排或补写事件。工作区由原场景 fixture 以固定 Git 元数据重建到源提交，再应用 Agent 在截断点前
已经产生的真实 patch。

## 反迎合约束

1. continuation prompt 只要求继续工作，不描述 S1、节点数量、待修代码或预期答案；
2. pytest 的失败输出、未知 plan 根因和后续修复不在前缀中；
3. 不调低 token 阈值，不增加 Runtime 激活触发器，不修改 S1 eligibility；
4. Standard、前一版本和候选版本共享同一前缀、工作区、prompt、模型、容器和 validator；
5. 任一 hash、Git tree、RPC 或初始失败状态不匹配，整次运行标记为 harness invalid，不能计入收益。

`sample.json` 是机器可读合同，`rollout-prefix.jsonl.gz` 是不可改写的自然轨迹证据。
