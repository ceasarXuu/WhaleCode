# client work 恢复十轮真实结果

- Date: 2026-08-16
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arm: `map-request`
- Repeat: 10 个独立 `repeat=1`，分两批各 5 个并行执行
- Subject: `560d07827`
- Ledger: `WAR-20260816-211759-CLIENT-WORK-RESTORE-R10`

## 1. 验收目标

扩大验证工作型序列恢复非空 client `tools[]` 后，首次请求是否稳定同时初始化 Map 并执行 client work；同时分别统计
`I03-ARG-SYNTAX`、`I04-FRONTIER-EARLY`、`I04-REDUNDANT-INFLIGHT` 和历史顶层 client Tool 逃逸。

Standard 侧按 right-only 设计跳过，本轮不形成 Standard 成本或效果对比。每轮最多 12 个 Provider 请求，零自动重试。

## 2. 逐轮结果

| Run | 首次请求 | 业务/外部/Map | Requests | Input | Cached | Uncached | Output | Agent wall | 独立异常 |
|---:|---|---|---:|---:|---:|---:|---:|---:|---|
| 1 | initialize + exec_command | passed | 6 | 84,627 | 78,976 | 5,651 | 1,708 | 18.887s | none |
| 2 | initialize + exec_command | passed | 7 | 101,798 | 94,080 | 7,718 | 1,958 | 22.375s | none |
| 3 | initialize + exec_command | passed | 9 | 136,050 | 127,104 | 8,946 | 2,456 | 29.626s | REDUNDANT-INFLIGHT |
| 4 | initialize + exec_command | passed | 8 | 118,932 | 110,464 | 8,468 | 2,637 | 29.144s | REDUNDANT-INFLIGHT |
| 5 | initialize + exec_command | passed | 6 | 85,210 | 78,592 | 6,618 | 1,806 | 19.676s | none |
| 6 | initialize + exec_command | passed | 8 | 119,791 | 111,488 | 8,303 | 2,455 | 26.224s | FRONTIER-EARLY |
| 7 | initialize + exec_command | passed | 7 | 103,400 | 94,848 | 8,552 | 2,328 | 24.485s | none |
| 8 | initialize + exec_command | passed | 8 | 118,224 | 109,952 | 8,272 | 2,136 | 23.012s | FRONTIER-EARLY |
| 9 | initialize + exec_command | passed | 9 | 136,715 | 128,512 | 8,203 | 2,467 | 26.682s | ARG-SYNTAX + FRONTIER-EARLY |
| 10 | initialize + exec_command | passed | 7 | 100,731 | 93,824 | 6,907 | 1,966 | 21.630s | FRONTIER-EARLY |
| **Total** | **work 10/10** | **10/10** | **75** | **1,105,478** | **1,027,840** | **77,638** | **21,917** | **241.741s** | **7 rejects / 6 runs** |
| **Mean** | - | - | **7.5** | **110,547.8** | **102,784** | **7,763.8** | **2,191.7** | **24.174s** | - |
| **Median** | - | - | **7.5** | **110,812** | **102,400** | **8,237.5** | **2,232** | **23.749s** | - |

十轮首个 Function Call 均为 `type=initialize_and_work`，且携带一个绑定首个 Work 节点的 `exec_command`。Map-only 空初始化、
顶层 client Tool 逃逸和 Provider 自动重试均为 0/10。十轮均修改 `src/tax_calc.py`，Agent complete、业务结果、公开验证、
隐藏 oracle 和 Map 闭环均为 10/10。

十个 wrapper 进程因 right-only 跳过 Standard、`repeats_lt_3` 和未启用 aggregate 而按 E2 utility gate 返回 1；每个目标侧均为
`phase=completed`、`run_validity=valid`、`exit_code=0`。该 wrapper 状态不属于 Agent 或业务失败。

## 3. 独立异常

| Stable ID | 本批事件 | 影响轮次 | 修正路径 |
|---|---:|---:|---|
| I03-ARG-SYNTAX | 1 | 1/10 | Run 9 收到准确 syntax reject 后，下一请求提交合法 JSON |
| I04-FRONTIER-EARLY | 4 | 4/10 | Run 6/8/9/10 在 `fix` 未完成时选择 Waiting `verify`；下一请求完成 `fix` 并继续 |
| I04-REDUNDANT-INFLIGHT | 2 | 2/10 | Run 3/4 同批显式设置 `fix:in_flight` 并执行 Tool；移除冗余转换后继续 |

七次拒绝均发生在成功初始化并执行首个 client work 之后，Runtime 均在副作用前拒绝并返回准确层级，Agent 最终均恢复。
Run 3 在状态拒绝后额外读取一次 Map；Run 4 先单独更新 Map、下一请求再执行 Tool。它们说明异常恢复还会放大请求，但不能把
全部请求差异机械归因于七次拒绝。

扩大样本后的独立登记和累计频率见
[`67-repeat3-independent-anomaly-register.md`](67-repeat3-independent-anomaly-register.md)。

## 4. 成本与缓存

- 加权全量缓存命中率：`1,027,840 / 1,105,478 = 92.98%`。
- Request 2+：`906,240 / 982,786 = 92.21%`。
- 十轮均无 zero-cache、same-shape zero、Tool choice 切换、cache-shape 切换或 Provider retry。
- Runtime capability identity 十轮一致：`571f3cfe8d9e3686e95423330c0de1af45ea300d257b5af4146082981b7acbfe`。
- 按冻结单价估算总费用：`0.14202880 CNY`，低于 2 元授权上限。

本轮不是缓存专用双臂 runner，不能用于晋升 accepted baseline；但 trace 足以排除 Tool shape 切换或缓存失效导致本轮异常。

## 5. Map 与判断

- 每轮均为 5 nodes / 4 edges，Root、3 个 Work 和明确 Finish 最终 Completed，open leaf 为 0。
- 无孤立节点、边顺序违规、Store 错误、Provider 聚合冲突或顶层 client Tool 逃逸。
- Map-only 空初始化由前批 0/3 扩大验证为本批 0/10；结合两批，首次合法初始化并执行 client work 累计为 13/13。
- 该结构恢复可以收敛为通过，但 I03/I04 不能关闭：ARG-SYNTAX、frontier 误选和冗余状态转换仍是可重复的独立行为问题。
- 本轮不增加 Runtime 语义干预；后续应先检查 Tool 合法序列表达和 Agent 可见合同，而不是因异常频率直接扩大状态机职责。

## 6. 证据

- `target/r8-client-work-restoration/repeat10-{1..5}/single-file-fast-fix/20260816-212041-*`
- `target/r8-client-work-restoration/repeat10-{6..10}/single-file-fast-fix/20260816-212219-*`
- 每个 root 均包含 `performance-observation.{json,md}`、`request-facts.json`、`provider-wire-trace.jsonl` 和 `rollout.jsonl`。
