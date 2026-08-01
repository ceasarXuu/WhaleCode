# R6 Phase F5.0 Bootstrap 合同因果实验结果

- 日期：2026-07-17
- 状态：Complete / H-008 Refuted / F5.0b Pending
- 范围：仅 F5.0 诊断与基线冻结
- 代码提交：`a6aad7b49`、`67e2605a1`、`741cea900`、`40e4fc6a4`
- 计划：`18-r6-phase-f5-cost-regression-repair-plan.md`
- COE：`coe/2026-07-16-18-52-r6-phase-f-context-cost.md`

## 1. 结论

F5.0 已完成，但结果否定了原 H-008：`finish.goal` 不是由完整 lifecycle schema 过宽或顶层描述显著性不足造成。
同一生产 builder 导出的三臂中，A、B、C 分别有 6/6、5/6、6/6 首次初始化携带非法 `finish.goal`，
其余完整校验字段均合法。

恢复 bootstrap-only 工具面可以继续作为 H-007 的 hard-state 对齐和 schema 成本修复，但不能再宣称它会解决
首次初始化重试。当前证据更接近“`strict=false` 下 Finish 对象与 Root/Work 节点的结构相似性导致模型补齐
`goal`”，该方向作为 H-011 保持 investigating，尚未进入生产修复。

## 2. 实验设计

Rust benchmark example 直接调用生产唯一 owner `create_taskspace_control_tool()` 导出完整 schema；PowerShell probe
只做机械裁臂，不复制手写协议：

| Arm | Schema | Tool description | Schema bytes |
|---|---|---|---:|
| A | 完整 7 类 lifecycle | 当前通用描述 | 9,427 |
| B | 仅保留 `initialize_map` | 与 A 完全相同 | 4,406 |
| C | 仅保留 `initialize_map` | 明确 `Finish` 只接受 `node_id` | 4,041 |

简单、复杂任务每臂各 3 次，共 18 次；`temperature=0`、named `taskspace_control` 和 prompt 固定，arm 使用
A/B/C、B/C/A、C/A/B 轮换。DeepSeek 不支持 named tool choice 与 thinking 同时启用，因此 probe 与生产
`client.rs` 一致使用 `thinking=disabled`，没有把 provider 400 计入正式结果。

正式原始证据：

- `target/r6-f5-bootstrap-ab/20260717-live-03/provider-capability.json`
- `target/r6-f5-bootstrap-ab/20260717-live-03/probe-events.jsonl`

`live-02` 是初次有效重复观察，A/B/C 为 6/6、6/6、6/6；发现深层字段校验覆盖不足后没有直接用它收口，
补齐 additional Work、edges 和 continuation action 校验并重跑得到 `live-03`。

首次 `live-01` 的 18 个请求均被 provider 以 `Thinking mode does not support this tool_choice` 拒绝，属于探针
请求整形错误，已保留但从实验中排除。

## 3. 三臂结果

| Arm | HTTP/parsed | `finish.goal` | 其他字段错误 | Input total/mean/median | Uncached total/mean/median | Duration ms total/mean/median | Request 2+ cache |
|---|---:|---:|---:|---:|---:|---:|---:|
| A | 6/6 | 6/6 | 0 | 17,208 / 2,868 / 2,868 | 312 / 52 / 52 | 15,600 / 2,600 / 2,585.5 | 98.19% |
| B | 6/6 | 5/6 | 0 | 8,622 / 1,437 / 1,437 | 174 / 29 / 29 | 15,546 / 2,591 / 2,555.5 | 97.98% |
| C | 6/6 | 6/6 | 0 | 8,226 / 1,371 / 1,371 | 546 / 91 / 91 | 18,210 / 3,035 / 2,804 | 93.36% |

三臂每次都只调用一次 `taskspace_control`，参数都能解析为 `initialize_map`。唯一错误路径恒为
`unexpected:finish.goal`。C 已同时具备 bootstrap-only schema、Finish 自身的 node-id-only schema 和显式顶层描述，
仍然 6/6 失败；B 也只有 1/6 合法，远未达到错误不高于 1/6 的支持门。因此 schema breadth 与
description salience 两个解释均被反证。两轮有效观察合并为 A=12/12、B=11/12、C=12/12。

## 4. 基线冻结

F5 final 使用总和、均值、中位数、request-2+ cache、terminal uncached 和
`weighted input = uncached + cached / 5`。下表与 Phase E/F4 原始 performance observation 逐项对账：

| Sample | Baseline | Requests total/mean/median | Input total/mean/median | Uncached total/mean/median | Weighted total/mean/median | Request 2+ cache | Terminal uncached |
|---|---|---:|---:|---:|---:|---:|---:|
| simple | Phase E | 28 / 9.33 / 9 | 213,502 / 71,167 / 68,001 | 20,862 / 6,954 / 6,902 | 59,390 / 19,796.67 / 19,361 | 89.74% | 11,262 |
| simple | F4 | 43 / 14.33 / 14 | 428,351 / 142,784 / 141,345 | 66,367 / 22,122 / 21,983 | 138,763.8 / 46,254.6 / 47,034.6 | 84.19% | 27,532 |
| complex | Phase E | 43 / 14.33 / 11 | 536,250 / 178,750 / 132,423 | 46,394 / 15,465 / 12,999 | 144,365.2 / 48,121.73 / 36,883.8 | 91.18% | 26,398 |
| complex | F4 | 58 / 19.33 / 17 | 863,940 / 287,980 / 241,537 | 98,116 / 32,705 / 32,897 | 251,280.8 / 83,760.27 / 71,553 | 88.56% | 44,368 |

Phase E 证据路径为 `e6-final-current/single-file-fast-fix` 和 `e6-live-path-fix-final/subscription-billing-repair`；
F4 路径为 `r6-phase-f4/single-file-fast-fix` 和 `r6-phase-f4-final/subscription-billing-repair`。表中 cached 由原始
performance observation 直接读取，weighted 只做冻结公式计算，没有更换观测口径。

## 5. R5 行为迁移清单

| R5 已验证不变量 | R6 当前状态 | F5 归属 |
|---|---|---|
| 非终态 finish 必须与非空 next actions 同一 control call | 已退化为 standalone `complete` + sibling call | F5.2 `complete_then_continue` |
| finish、next binding、actions 有明确顺序 | 只在 initialize/bind continuation 中存在 | F5.2 原子 state plan + ToolRouter |
| patch 只有一个明确槽位 | R6 continuation 已保留 | F5.2 必须继续保持 |
| terminal finish 与最终总结同一 carrier | R6 `finish_end` 已成立 | F5.2 `complete_then_end` 复用该 envelope |
| nested failure 忠实返回并停止 tail | initialize/bind 已成立 | F5.2 handoff 必须继承 |

迁移判断以行为不变量为准，不以 R5/R6 类型名称或领域模型是否已经迁移代替能力对账。

## 6. 验证与收益

| Gate | Result |
|---|---:|
| `codex-tools taskspace_control --lib` | PASS |
| schema exporter | 7 个 lifecycle 变体，首项 `initialize_map` |
| probe fixture | 支持分支与反证分支均 PASS |
| sensitive goal/reasoning 原文不落盘 | PASS |
| live provider | 18/18 HTTP 200，18/18 单 control call，18/18 可解析 |
| Phase E/F4 基线对账 | 100% |

明确收益有两项：第一，H-008 已被可复现实验否定，避免把无效的 bootstrap-only 变更误当作初始化修复；第二，
B/C 的输入约为 A 的一半，独立支持 H-007 的 schema 成本方向，但这只是 provider probe 成本，不等同于端到端收益。

## 7. 下一步

先执行 F5.0b，只比较 Finish identity 的 wire shape，不修改 production。H-011 证据门通过后，F5.0c 才允许切换
生产合同；若没有任一形态把错误降到不高于 1/6，则保持 investigating，不增加 Runtime 纠错或 projection 提示。
完成后再进入只处理 H-007 的 F5.1 hard-state 工具面。
