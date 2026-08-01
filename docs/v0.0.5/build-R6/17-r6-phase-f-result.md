# R6 Phase F 上下文唯一性与成本收敛结果

- 日期：2026-07-16
- 状态：F0-F4 机制完成 / 端到端成本门失败 / F5.0-F5.0b 已完成、F5.0c 待执行
- 范围：F0、F1、F2、F3、F3.5、F4
- 最终代码提交：`726d3298b`
- 计划：`16-r6-phase-f-context-cost-plan.md`
- COE：`coe/2026-07-16-18-52-r6-phase-f-context-cost.md`

## 1. 结论

Phase F 的 F0-F4 六个机制阶段均已实现并通过各自的局部门禁，但整体 outcome 不能计为完成。后续审计证明
Phase F final 相对 Phase E 的 simple request/input 增加 53.6%/100.6%，complex 增加 34.9%/61.1%；因此
“6/6 = 100%”只适用于原局部实施项，不适用于 Phase F 的端到端成本目标。

正式 Docker 结果为：simple 与最终 complex 各 3 对 Standard/R6，12/12 side 全部通过 public/hidden
validator；6/6 R6 Map 均由 Agent 调用 `finish_end` 闭合，节点/边骨架完整，语义替换为 0。缓存结构性故障
已修复：F3 的约 13% request-2+ 命中与 0% message prefix，提高到 F4 simple 84.19%/85.00%、
complex 88.56%/89.09%。

成本仍高于 Standard，且显著劣于 Phase E。根因已收敛到负收益 immutable lifecycle schema、R6 迁移丢失的
complete handoff carrier、稳定 bootstrap 参数回归和验收门缺失。Phase F 已重开，先执行
`18-r6-phase-f5-cost-regression-repair-plan.md`；不得通过 projection 语义裁剪或 Runtime 替 Agent 决策压低指标。

## 2. 阶段完成度

| Stage | 目标 | 实现与验证 | 得分 |
|---|---|---|---:|
| F0 | provider payload 分区可观测 | 八类 section 对账、hash/LCP、observer/harness fixture | 100% |
| F1 | 当前 Map 单一 provider owner | control result 删除完整 `map_state`，只保留 canonical delta/ref | 100% |
| F2 | 稳定 tool contract | 全生命周期 13 tools、单一 tools hash；`required+thinking` 证据化 HOLD | 100% |
| F3 | Agent 声明的机械 continuation | init/bind/mutation sequence、首错停止、单 patch 预检 | 100% |
| F3.5 | 恢复严格 provider 前缀 | 固定 epoch baseline 锚点，后续原始 delta journal 顺序追加 | 100% |
| F4 | deterministic + Docker 正式门禁 | 定向回归、simple 3×、最终 complex 3×、malformed live 闭环 | 100% |

## 3. 正式矩阵

单元格格式为 `总和 / 均值 / 中位数`。缓存为三轮加权 request-2+ 命中；Prefix 为跨轮精确消息前缀。

| Sample | Mode | Solved | Requests | Wall(s) | Input | Cached | Uncached | Output | Cache 2+ | Prefix |
|---|---|---:|---|---|---|---|---|---|---:|---:|
| simple | Standard | 3/3 | 20 / 6.67 / 7 | 48.49 / 16.16 / 16.13 | 138,330 / 46,110 / 48,677 | 132,224 / 44,075 / 46,592 | 6,106 / 2,035 / 2,085 | 4,589 / 1,530 / 1,481 | 95.23% | 100.00% |
| simple | R6 | 3/3 | 43 / 14.33 / 14 | 118.81 / 39.60 / 36.95 | 428,351 / 142,784 / 141,345 | 361,984 / 120,661 / 117,888 | 66,367 / 22,122 / 21,983 | 11,771 / 3,924 / 3,356 | 84.19% | 85.00% |
| complex | Standard | 3/3 | 41 / 13.67 / 14 | 167.56 / 55.85 / 51.40 | 444,591 / 148,197 / 154,901 | 425,600 / 141,867 / 150,272 | 18,991 / 6,330 / 6,633 | 16,592 / 5,531 / 5,340 | 95.59% | 100.00% |
| complex | R6 | 3/3 | 58 / 19.33 / 17 | 212.29 / 70.76 / 65.05 | 863,940 / 287,980 / 241,537 | 765,824 / 255,275 / 212,480 | 98,116 / 32,705 / 32,897 | 20,936 / 6,979 / 6,866 | 88.56% | 89.09% |

运行证据：

- simple：`target/r6-phase-f4/single-file-fast-fix/20260716-231150-152`
- final complex：`target/r6-phase-f4-final/subscription-billing-repair/20260716-233621-580`
- strict parser smoke：`target/r6-phase-f4-strict-parser/subscription-billing-repair/20260716-233221-635`

simple 矩阵早于 H-006 strict parser 提交，但其全部原始 control 参数均为严格合法 JSON；H-006 只改变 malformed
输入。合法路径由更新后的 control/sequence fixture 和复杂矩阵再次覆盖，因此不重复消耗 simple 三轮。

## 4. Map 与终结

| Sample | R6 runs | Nodes | Edges | Open | `finish_end` | Projection max | Semantic retention/rewrite |
|---|---:|---:|---:|---:|---:|---:|---|
| simple | 3 | 15 | 12 | 0 | 3/3 | 1/request | 100% / 0% |
| complex | 3 | 15 | 12 | 0 | 3/3 | 1/request | 100% / 0% |

每张 Map 均为 Root -> Work 路径 -> Finish 的有向依赖图；Root/Finish 同 terminal revision 闭合。control
反馈不再重复完整 Map，active projection 只有一个 epoch baseline，baseline 后的 call/result/tool feedback 按原顺序保存。

## 5. 工程收益

| 收益 | Baseline | 目标 | 观测结果 | 验证 |
|---|---:|---:|---:|---|
| control 当前状态去重 | init result 1,018 B | 降低至少 30% | 539 B，-47.1% | deterministic fixture |
| nested/tool schema 去重 | 35,648 B/request | 单一 owner | 24,449 B/request，-31.4%（F2 同构） | wire section trace |
| provider 前缀恢复 | F3 prefix 0%；cache 约 13% | 两项均 >=80% | simple 85.00%/84.19%；complex 89.09%/88.56% | final wire trace |
| correctness/terminal | 不允许性能换正确性 | 100% | 12/12 side solved；6/6 R6 手动终结 | Docker public/hidden oracle |
| malformed feedback 身份 | 空 call id 可 panic/丢反馈 | call/output 可配对 | 原 call id、失败原文、`success=false`；无 orphan | session test + live trace |
| control JSON 保真 | malformed 尾部可被静默忽略 | 完整 JSON 文档 | malformed live 返回零提交，后续新 call 纠正 | H-006 live replay |

## 6. H-006 收口

F4 首轮 complex 发现：`serde_path_to_error::deserialize` 可读取合法首值，但旧代码未调用
`Deserializer::end()`，因此一个尾部多余 `}` 的 bind+patch call 被静默执行；Event Store 和 observer 却忠实记录为
malformed，形成同一 call 的执行/回放分歧。

修复后 parser 必须消费完整 JSON 文档，不修复、不截断、不猜测 Agent 参数。最终 complex pair-002 自然产生了
continuation 错误闭合的 call，Runtime 返回同 call id 的 `protocol_failed`、`state_commit=false`、
`partial_commit=0`，未执行嵌套 patch；Agent 随后用新 call 完成任务。该结果证明严格反馈不会形成 reject loop。

## 7. 验证矩阵

| Gate | Result |
|---|---:|
| `codex-tools --lib` | 141 passed，1 ignored |
| `codex-protocol --lib` | 197 passed |
| action map | 67 passed |
| TaskSpace control | 25 passed |
| sequence | 13 passed |
| session | 183 passed |
| replay + rollout reconstruction | 18 + 33 passed |
| provider budget/payload/sections/epoch | 8 + 1 + 8 + 2 passed |
| cost/performance/native-control/rooted-DAG/harness | PASS |
| `just fix -p codex-core`、`just fmt` | PASS |
| `cargo build -p codex-cli --bin whale --locked` + attestation | PASS |

首次 session 回归因 `.env.local` 相对路径多写一层，Guardian 2 项按设计失败；使用正确路径加载 key 后 183/183
通过。该失败不是代码回归，也没有通过 fallback 绕过。

## 8. 未完成工作

| Item | 状态 | 原因与影响 | 后续归属 |
|---|---|---|---|
| request 放大 | 未解决 | F final 相对 E simple/complex +53.6%/+34.9% | Phase F5.1/F5.2 |
| uncached input 放大 | 未解决 | F final 相对 E simple/complex +218.1%/+111.5% | Phase F5.1/F5.3 |
| bootstrap 首次参数失败 | 已定位、未实施 | F5.0b D/E/F=5/0/1；E 合同胜出 | Phase F5.0c |
| standalone complete | 未解决 | F3 依赖 sibling calls，正式运行 multi-control adoption=0/6 | Phase F5.2 |
| `required+thinking` | HOLD | provider 返回 `thinking_tool_choice_incompatible`；不能用缓存换思考能力 | provider 能力变化后重测 |
| 长 Map 详情压缩 | 未开始 | Phase F 禁止语义裁剪；长期上下文上限尚未解决 | Phase G 单策略实验 |
| 骨架本身超限 | 未开始 | 全局 skeleton 不能分页后假装仍有全局视野 | 后续独立专项 |

## 9. 下一步

当前提交不能冻结为健康 R6-B0。先按 `18-r6-phase-f5-cost-regression-repair-plan.md` 完成 F5.0-F5.3；只有
correctness、handoff、bootstrap、request、input、uncached 和 weighted input 全部通过 Phase E 门，才冻结新的 R6-B0
并进入 Phase G。
