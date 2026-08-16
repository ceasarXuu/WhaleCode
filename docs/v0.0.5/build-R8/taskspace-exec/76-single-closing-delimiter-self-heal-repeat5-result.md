# 单闭合符号自愈五轮真实验证

- Date: 2026-08-17
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arm: `map-request`
- Repeat: 5 个独立 `repeat=1`，并行执行
- Subject commit: `2c2144e73`
- Binary SHA-256: `f4779fadd2c3f53198fc85fe2a1930559b3e4d0c2d7c33cb178f1f5b790f77b6`
- Ledger: `WAR-20260817-034839-SELF-HEAL-R5`
- Baseline: [`74-base-lifecycle-prominence-repeat5-result.md`](74-base-lifecycle-prominence-repeat5-result.md)

## 1. 逐轮结果

| Run | 结果 | Requests | Input | Cached | Uncached | Output | Agent wall | Request 2+ cache | 自愈 / 拒绝 | Map |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | passed | 7 | 108,426 | 102,528 | 5,898 | 1,992 | 17.245s | 93.93% | 0 / 0 | 5 nodes / 4 edges |
| 2 | passed | 7 | 109,158 | 101,376 | 7,782 | 2,122 | 18.424s | 92.03% | 0 / 0 | 5 nodes / 4 edges |
| 3 | passed | 7 | 107,140 | 98,816 | 8,324 | 2,023 | 18.051s | 91.28% | 0 / 0 | 5 nodes / 4 edges |
| 4 | passed | 7 | 110,613 | 102,144 | 8,469 | 2,585 | 21.462s | 91.44% | 1 insert `}` / 0 | 5 nodes / 4 edges |
| 5 | passed | 6 | 93,113 | 86,272 | 6,841 | 2,634 | 23.335s | 91.61% | 0 / 0 | 5 nodes / 4 edges |
| **Total** | **5/5** | **34** | **528,450** | **491,136** | **37,314** | **11,356** | **98.517s** | **92.07% weighted** | **1 / 0** | - |
| **Mean** | - | **6.8** | **105,690.0** | **98,227.2** | **7,462.8** | **2,271.2** | **19.703s** | - | - | - |
| **Median** | - | **7** | **108,426** | **101,376** | **7,782** | **2,122** | **18.424s** | - | - | - |

按冻结价格估算费用为 `0.06984872 CNY`。五个 runner 因只执行 right side 而以 pair 评分退出码 `1` 结束；side artifacts
证明每轮 Agent completion、业务修复、公开验证、隐藏 oracle 和 Map 闭合全部通过，实际 side exit code 均为 `0`。

## 2. 在线自愈证据

Run 4 的 `call_00_tyEfYIcbmh6xlcaeaJmv5659` 自然生成了缺少一个外层 `}` 的 `update_and_work`：

- 审计事件：`repair_operation="insert"`、`delimiter=}`、`byte_index=528`；
- 原始摘要：`1c90d3612a744ba118479d80da805c590f6cf33cf7fd36532e92fa388d89c0c6`；
- 修复摘要：`a4ff9330a20c3031de4d86d416786c2fa9e3ad13c3b56e6bdeab1c63aeb951fb`；
- rollout 中正式 Function Call 参数摘要与修复摘要相同，错误版没有进入正式历史；
- 同一 outer call 继续通过 preflight、持久化和 `apply_patch@fix`，没有 syntax reject 或额外 Provider 请求。

这证明落账前替换链和原有“插入缺失闭合符号”分支在线生效。本轮没有自然生成单个多余 `}` / `]`，因此新加入的删除分支
仍由真实历史坏例、与 Agent 成功重试逐字匹配及确定性测试证明，不能宣称已在线命中。

## 3. 行动与 Map

- 五轮都建立 `root -> inspect -> fix -> verify -> finish`，最终 5 个节点全部 `completed`、0 open leaves；
- 首请求均为 `initialize_and_work + exec_command@inspect`；
- 五轮均在 `update_and_work` 中完成父节点并继续下游工作，最终使用 `update_and_finish`；
- 总计 29 个 `taskspace_exec`、24 个 client actions；无 sequence preflight、参数合同、状态机或普通 Tool 失败；
- Run 2 有一次已处于 `in_flight` 的 `fix` 被重复声明为 `in_flight`，但没有产生拒绝或额外请求；作为低价值冗余继续观察，
  不据单次行为增加 Runtime 约束。

## 4. 与上一批比较

| 指标 | 上一批 | 当前批 | 变化 |
|---|---:|---:|---:|
| 业务 / Map 通过 | 5/5 | 5/5 | 持平 |
| Syntax reject | 1 | 0 | -1 |
| 自愈在线命中 | 0 | 1 | +1 |
| Requests | 35 | 34 | -2.86% |
| Input | 544,904 | 528,450 | -3.02% |
| Uncached input | 78,472 | 37,314 | -52.45% |
| Output | 11,093 | 11,356 | +2.37% |
| Agent wall | 99.573s | 98.517s | -1.06% |
| Request 2+ cache | 91.21% | 92.07% | +0.86 pp |
| 估算费用 | 0.10998664 CNY | 0.06984872 CNY | -36.49% |

当前批没有性能回归，但缓存和成本改善不能全部归因于自愈：两批是独立随机运行，且删除分支未命中。可以确定的直接收益只有
Run 4 的坏参数没有形成拒绝和重试，其余差异保留为批次观测。

## 5. 证据

- `target/r8-self-heal/repeat5-{1..5}/single-file-fast-fix/20260817-035141-*`
- 免费预检：`target/r8-self-heal/repeat5-preflight/single-file-fast-fix/20260817-035122-104`
