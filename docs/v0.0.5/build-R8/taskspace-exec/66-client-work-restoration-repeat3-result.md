# client work 恢复三轮真实结果

- Date: 2026-08-16
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arm: `map-request`
- Repeat: 3 个独立 `repeat=1`，并行执行
- Subject: `560d07827`
- Ledger: `WAR-20260816-210454-CLIENT-WORK-RESTORE-R3`

## 1. 验收目标

验证工作型序列恢复非空 client `tools[]` 后，C2 十轮中 4/10 的“首次只建 Map、不执行 work”是否消失；同时观察业务结果、
请求成本、缓存、Map 和其他协议异常。Standard 侧按 right-only 设计跳过，本轮不形成 Standard 对比。

## 2. 逐轮结果

| Run | 首次请求 | 首次接受 | Agent | 业务/外部 | Requests | Input | Cached | Uncached | Output | Agent wall |
|---:|---|---|---|---|---:|---:|---:|---:|---:|---:|
| 1 | initialize + exec_command | 是 | complete | passed | 9 | 132,670 | 117,632 | 15,038 | 2,104 | 21.775s |
| 2 | initialize + exec_command | 是 | complete | passed | 6 | 84,866 | 70,912 | 13,954 | 1,738 | 16.430s |
| 3 | initialize + exec_command | 是 | complete | passed | 8 | 120,533 | 104,064 | 16,469 | 2,975 | 25.708s |
| **Total** | **work 3/3** | **3/3** | **3/3** | **3/3** | **23** | **338,069** | **292,608** | **45,461** | **6,817** | **63.913s** |
| **Mean** | - | - | - | - | **7.67** | **112,689.67** | **97,536** | **15,153.67** | **2,272.33** | **21.304s** |
| **Median** | - | - | - | - | **8** | **120,533** | **104,064** | **15,038** | **2,104** | **21.775s** |

三轮都修改 `src/tax_calc.py`，公开测试、隐藏 oracle 和最终 Map 闭环全部通过。每轮首个 Function Call 都是
`type=initialize_and_work`，并携带绑定 `inspect` 节点的 `exec_command`；没有 Map-only 请求，也没有因此产生补交 Map 的额外轮次。

三个 wrapper 进程因 right-only 跳过 Standard、`repeats_lt_3` 和未启用 aggregate 而按 E2 utility gate 返回 1；每个目标侧
`run-status.json` 与 `sample-status.json` 均为 `phase=completed`、`run_validity=valid`、`exit_code=0`。该 wrapper 状态不属于
Agent 或业务失败。

## 3. 残余异常

- Run 1：一次直接在 Waiting 的 `fix` 节点执行 work，被硬规则零副作用拒绝；一次 `taskspace_exec.arguments` JSON 缺少合法分隔，
  由 parser 零副作用拒绝。Agent 均在下一请求修正。
- Run 3：两次把即将执行 Tool 的 Ready 节点同时显式改为 `in_flight`，触发非法状态转换并零副作用拒绝；Agent 均删除冗余转换后
  修正。
- Run 2：没有 TaskSpace Exec 拒绝。
- 未复现历史的顶层 client Tool 逃逸，但 3 次不足以关闭该独立问题。

这些异常不属于本次“首次无 client work”回归。独立归类和稳定标识见
[`67-repeat3-independent-anomaly-register.md`](67-repeat3-independent-anomaly-register.md)。

## 4. 成本与缓存

- 加权全量缓存命中率：`292,608 / 338,069 = 86.55%`。
- Request 2+：`280,448 / 301,264 = 93.09%`。
- 三轮均无 zero-cache、same-shape zero、Tool choice 切换或 cache-shape 切换。
- Tool schema hash 三轮一致：`826dee300da11df9fd350fcabec6cc5b8d5fbd5e289a08a9d44484d777a57735`。
- 按冻结单价估算总费用：`0.06494716 CNY`，低于 1 元授权上限。

全量命中率受本次新 Tool schema 的首请求冷形状影响；Request 2+ 与 C2 十轮历史值 92.71% 同量级。本轮不是缓存专用双臂
runner，不能用于晋升 accepted baseline。

## 5. Map

- Run 1、2：4 nodes / 3 edges；Run 3：5 nodes / 4 edges。
- 三轮 Root、Work 和明确 Finish 均最终 Completed，open leaf 为 0。
- 无孤立节点、边顺序违规、Store 错误或 Provider 聚合冲突。

## 6. 判断

工作型序列的 client work 结构前置条件已在线生效：目标失败从 C2 的 4/10 降为本轮 0/3，且没有以业务失败、Map 失败或
warm-cache 回归换取。该子问题可以视为修复通过；I03 仍因本轮和历史中的其他动作组织问题保持 verifying。

## 7. 证据

- `target/r8-client-work-restoration/repeat3-1/single-file-fast-fix/20260816-210852-165`
- `target/r8-client-work-restoration/repeat3-2/single-file-fast-fix/20260816-210852-096`
- `target/r8-client-work-restoration/repeat3-3/single-file-fast-fix/20260816-210852-146`
- 每个 root 均包含 `performance-observation.{json,md}`、`request-facts.json`、`provider-wire-trace.jsonl` 和 `rollout.jsonl`。
