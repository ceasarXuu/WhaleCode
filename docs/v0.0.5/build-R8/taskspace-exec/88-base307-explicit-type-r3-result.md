# Base 3.0.7 显式序列类型真实验证

- 日期：2026-08-20
- 运行账本：`WAR-20260820-050331-R8-BASE307-TYPE-R5`
- 被测提交：`e246ca6a2`
- 模型：`deepseek-v4-flash`
- 样本：`release-dispatch-repair`

## 1. 验证目标

Base `3.0.7` 只新增一条硬正确性说明：每次 `taskspace_exec` 都必须显式提供顶层 `type`，由它选择一个
schema 已定义的合法序列；Runtime 不替 Agent 推断缺失的 `type`。本轮观察三件事：

1. 缺失 `type` 是否停止复发。
2. 业务、隐藏验收和 Map 闭环是否回归。
3. 请求、input 与缓存是否出现新的结构性异常。

## 2. 计划与实际矩阵

计划为 Standard `repeat=1`、TaskSpace `repeat=5`。Pair runner 会在物理 side 上交替逻辑模式，实际形成
Standard `repeat=3`、TaskSpace `repeat=3`。发现偏差后没有追加运行，因此本报告只按实际矩阵下结论，不把它表述为
TaskSpace `repeat=5`。

## 3. 逐轮结果

| 模式 | 轮次 | 请求 | Input | Cached | Uncached | Output | Request 2+ cache | Agent wall | Exec/type | 结果 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| Standard | 1 | 8 | 111,597 | 108,288 | 3,309 | 3,171 | 96.92% | 27.934s | - | 业务/公开/隐藏通过 |
| Standard | 2 | 9 | 132,808 | 129,536 | 3,272 | 3,795 | 97.38% | 28.971s | - | 业务/公开/隐藏通过；1 次普通 Tool 失败后恢复 |
| Standard | 3 | 7 | 90,174 | 86,016 | 4,158 | 2,716 | 94.85% | 22.221s | - | 业务/公开/隐藏通过 |
| TaskSpace | 1 | 7 | 114,926 | 94,464 | 20,462 | 3,711 | 88.70% | 27.296s | 6/6 | 业务/公开/隐藏/Map 通过 |
| TaskSpace | 2 | 8 | 129,674 | 117,888 | 11,786 | 3,998 | 89.99% | 30.367s | 7/7 | 业务/公开/隐藏/Map 通过 |
| TaskSpace | 3 | 6 | 98,760 | 87,680 | 11,080 | 3,699 | 87.21% | 28.161s | 5/5 | 业务/公开/隐藏/Map 通过 |

## 4. 聚合对比

| 模式 | Runs | Success | Requests | Input | Cached | Uncached | Output | Request 2+ cache | Agent wall | CNY |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Standard | 3 | 3/3 | 24 | 334,579 | 323,840 | 10,739 | 9,682 | 96.56% | 79.126s | 0.03657980 |
| TaskSpace map-request | 3 | 3/3 | 21 | 343,360 | 300,032 | 43,328 | 11,408 | 88.77% | 85.824s | 0.07214464 |

TaskSpace 请求为 Standard 的 `0.875x`，平均每轮 input 为 `1.026x`，费用为 `1.972x`。费用差异主要来自
TaskSpace 的未缓存 input，而不是请求放大。该结果与 map-request 的已知协议和 Map 成本一致，未显示 Base `3.0.7`
新增一句话造成新的 input 结构异常。

## 5. 结论边界

- 三轮 TaskSpace 共 18 次 `taskspace_exec`，`18/18` 显式携带 `type`，缺失、语法、合同、状态和执行拒绝均为 0。
- 同一样本的前一批 Base `3.0.6` 为 `1/3` 运行发生缺失 `type`；本轮为 `0/3`。这支持把 Base `3.0.7`
  作为当前候选保留。
- TaskSpace 业务、公开验证、hidden oracle 和 Map 闭环均为 `3/3`，未观察到正确性回归。
- 三轮不足以证明跨样本稳定，也不足以关闭整个 I03；I03 继续 `verifying`。
- 实际矩阵偏离计划，且不是专用缓存回归 runner，因此本轮不晋升 accepted cache baseline。

## 6. 观测工程发现

真实 Provider 运行前后的后处理暴露了三个 I07 问题。前两个已离线修复并提交：graph observer 忽略无 ID
占位项（`f31ea64e8`），audit collection 始终按数组保存（`4c9d5cf3d`）。全部四轮后续 Provider 运行完成后，最终
sample timing 汇总仍因非对象 timing item 失败；原始 wire、usage、业务和 Map 证据完整，因此不影响本报告复算，但该
后处理缺口继续归 I07，不能宣称 I07 已关闭。

机器可读证据位于
`benchmarks/taskspace/r8/evidence/WAR-20260820-050331-R8-BASE307-TYPE-R5.json`。
