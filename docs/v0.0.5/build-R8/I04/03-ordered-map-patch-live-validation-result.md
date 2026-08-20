# R8 I04 有序 Map Patch 真实验收结果

- Date: 2026-08-19
- Subject commit: `5c8f428533e306581f62d5bb7b5179fc0f396fe6`
- Model: `deepseek-v4-flash`
- Status: cache passed / business passed / target sequence not observed

## 1. 执行范围

用户批准共三个 sample run，零重试：

1. `single-file-fast-fix`：Standard、map-request 各一次，用于缓存门禁；
2. `release-dispatch-repair`：只运行 map-request 一次，用于观察有序父子完成。

三次共 25 个 Provider 请求、395,991 input、356,864 cached、39,127 uncached、9,321 output，冻结价格估算
`CNY 0.06490628`，低于 `CNY 0.50` 总上限。

## 2. 缓存结果

| Arm | 业务 | 请求 | Input | Cached | Uncached | Output | Request 2+ hit |
|---|---:|---:|---:|---:|---:|---:|---:|
| Standard | PASS | 6 | 71,950 | 70,272 | 1,678 | 832 | 97.30% |
| map-request | PASS | 7 | 105,182 | 85,888 | 19,294 | 2,016 | 91.72% |

两臂 usage 覆盖率均为 100%，没有 retry、shape transition 或预算超限。结果满足当前缓存阈值，但仍需用户接受该精确
结果后才能晋升 accepted baseline。

## 3. 行为结果

复杂样本业务、公开测试、隐藏 Oracle 和 Map 闭合全部通过；12 个 Provider 请求，218,859 input、200,704 cached、
18,155 uncached、6,473 output，request 2+ hit 为 91.27%。Map 仍为五节点线性链。

实际收尾路径是：

1. `update_and_work`：完成 `fix`，在刚解锁的 `verify` 上运行全量测试；
2. `update_and_finish`：完成 `verify` 并关闭 Finish。

全程没有 `TransitionInvalid`，说明新实现未破坏正常父子交接；但 Agent 没有在同一 `node_patches[]` 中同时完成父节点和
刚解锁子节点，所以本轮没有直接命中新能力，生产验收继续保持 `verifying`。

本轮另发生一次顶层 `exec_command` 逃逸。Runtime 在执行前拒绝，Agent 下一请求改回 `taskspace_exec`；该事实归入 I03，
不归因于有序事务修复。

## 4. Runner 口径

行为 runner 退出码为 1，是因为通用 Pair E2 合同把有意跳过的 Standard 侧和 `repeat < 3` 视为 pair-level 不合格。
TaskSpace 右臂自身 `exec_exit_code=0`、`business_success=true`、公开验证和隐藏 Oracle 均通过。因此本轮是有效的单臂诊断
证据，但不能冒充完整双臂 E2。

## 5. 证据

- Cache result: `benchmarks/cache-regression/results/WAR-20260819-053954-CACHE-REGRESSION-E2191C90.json`
- Cache evidence: `benchmarks/cache-regression/evidence/WAR-20260819-053954-CACHE-REGRESSION-E2191C90/`
- Behavior pair: `target/whale-agent-runs/WAR-20260819-054148-R8-I04-ORDERED-PATCH-R1/release-dispatch-repair/20260819-054243-335/pair-001/pair-report.md`
- Behavior rollout: `target/whale-agent-runs/WAR-20260819-054148-R8-I04-ORDERED-PATCH-R1/release-dispatch-repair/20260819-054243-335/pair-001/right/artifacts/rollout.jsonl`
- Global ledger: `benchmarks/whale-agent-run-ledger.json`
