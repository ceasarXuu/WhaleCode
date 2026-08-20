# R8-I01 W9 map-always 单次复验结果

- 日期：2026-08-01
- 账本：`WAR-20260801-222316-R8-I01-W9-MA-1B64DB37`
- 产品提交：`9b49f6dc96ad553ab454fefc2c96c975a6838442`
- 运行提交：`7810f50e4`
- 模式：`map-always`
- Sample：`single-file-fast-fix`
- 实际执行：repeat 1；触发停止条件后未执行 repeat 2、3

## 1. 结论

本次没有复现 I01 的旧 revision 竞争问题。5 次成功的 response 只返回原
`taskspace_control` call 对应的一份 `TaskSpaceResponseResultV2`，唯一 continuation 字段为
`canonical_revision`；后续成功提交链为 `2 -> 4 -> 6 -> 8 -> 10`，`stale_revision=0`，旧
`TaskSpaceResponseFinalReceiptV1` 不存在。

本次运行整体仍判失败，但失败不属于 I01。Agent 已正确修复代码，公开测试与 hidden oracle 均通过；由于 10 次
真实 provider 请求中有 5 次 TaskSpace 零执行拒绝，Agent 在测试通过后尚未来得及提交 `finish_map`，第 11 次
本地请求尝试被 provider boundary 在上游 dispatch 前拒绝。

因此，本轮只支持“map-always 上 I01 修复方向成立”，不能支持“W9 已完成”或“map-always 产品路径已稳定”。
阻塞项归入现有 R8-I03；重复反馈和请求统计错误分别继续归入 I05/I07，不新增问题编号。

## 2. 请求路径

| 上游请求 | Agent 动作 | Runtime 结果 | 是否推进 |
|---:|---|---|---:|
| 1 | 单独 `exec_command` | `taskspace_control_required`，整响应零执行 | 否 |
| 2 | 单独 `initialize_and_execute`，但声明 1 个 sibling | `taskspace_action_count_mismatch` | 否 |
| 3 | 再次单独 `initialize_and_execute`，仍声明 1 个 sibling | `taskspace_action_count_mismatch` | 否 |
| 4 | `initialize_and_execute + exec_command` | Map 初始化并执行探索 | 是 |
| 5 | 单独 `exec_command` | `taskspace_control_required` | 否 |
| 6 | `execute + exec_command` | 读取 README、源码和测试 | 是 |
| 7 | 完成 `explore`，同时把测试动作仍归给 `explore` | `node_state_invalid`，零执行 | 否 |
| 8 | 完成 `explore`，测试动作归给 `fix` | 执行测试并确认失败 | 是 |
| 9 | `execute + apply_patch` | 修复 `round(..., 1)` 为 `round(..., 2)` | 是 |
| 10 | 完成 `fix`，测试动作归给 `verify` | 3 个测试全部通过 | 是 |
| 本地尝试 11 | 预期继续完成 Map | provider boundary 在上游 dispatch 前拒绝 | 否 |

真正的额外成本来自 5 个零推进尝试，其中前 4 个已经获得 provider 响应，第 11 个未发往 provider。请求 1、2、3、5
直接证明：当前 response-level manifest 能在执行前守住硬边界，但单个原生 Tool schema 无法结构化保证另一个
sibling Tool 必须同时出现。这是 I03 的既有合同缺口，不是 revision 反馈丢失。

## 3. 成本与证据

| 指标 | 实际值 | 证据口径 |
|---|---:|---|
| 真实上游请求 | 10 | provider boundary claimed/completed |
| 本地 wire attempts | 11 | 10 completed + 1 pre-dispatch rejected |
| Input tokens | 172,079 | 10 条 completed wire terminal 求和 |
| Cached input | 59,904 | 同上 |
| Uncached input | 112,175 | 同上 |
| 全请求缓存率 | 34.81% | cached / input；`map-always` 已知模式特征 |
| Output tokens | 4,649 | 10 条 completed wire terminal 求和 |
| 估算费用 | $0.0171739512 | 账本冻结价格公式 |
| Agent wall time | 44.671 s | `process-timing.json` |
| 总 pair 时间 | 49.884 s | `pair-timing.json` |
| 公开验证 / hidden oracle | passed / passed | Docker validators |

当前 `request-summary.json` 把同一 usage 统计成 19 requests、321,740 input、112,896 cached、9,098 output，
与 provider boundary 和 wire terminal 冲突。该报告不用于费用结算，作为 I07 的当前证据保留。

原始证据：

- `/tmp/wrun/20260801w9ma/a1/r-1/single-file-fast-fix/20260801-222449-350`
- `target/r8-i01-w9/WAR-20260801-222316-R8-I01-W9-MA-1B64DB37/map-always-r1`

## 4. 后续边界

1. I01 保持 `verifying`：map-always 单次 stale 目标通过，但 W9 要求的三 policy repeat-3 尚未完成。
2. 不通过放宽状态机、自动补 sibling、自动选择 node 或提高请求上限来伪装修复。
3. I03 必须单独设计：在普通 Tool 保持原生、Agent 显式声明 `node_id`、Runtime 只做 response 机械校验的前提下，
   解决跨 Tool 同响应组合的可生成性。
4. I05 后续删除拒绝事实的 developer message 副本；本轮同一拒绝同时存在配对 Tool output 和 developer factual
   message。
5. I07 后续修复 rejected local attempt 与 upstream dispatch 的核对模型，以及 rollout usage 双计数。
