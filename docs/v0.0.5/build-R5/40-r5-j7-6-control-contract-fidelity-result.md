# R5-J7.6 TaskSpace Control 契约忠实性修复结果

- Date: 2026-07-13
- Status: **IMPLEMENTATION COMPLETE；LIVE GATE PAUSED**
- Runtime contract commit: `31d5d68`
- Observer commits: `641bd54`、`5f2a09d`
- Binary SHA-256: `efacaf13320440af76f13f153c5369438081f929369af0ab70a2e014228ee134`
- Model: `deepseek-v4-flash`
- Substrate: Docker hard boundary，Standard/R5 各一次

## 1. 结论

H-025 原问题已修复并通过 live 证据门禁：初始化、非终态 finish 和 terminal 的成功反馈都返回已提交机械
身份；两个样本 `control_identity_missing_count=0`、`committed_repeat_finish_count=0`，且最终 task/map 均
completed、open node=0。J7.5 order 的“弱成功回执 -> 重复 finish -> draft node 膨胀 -> open Map”链路没有
复现。

J7.6 仍不能标记整体 live gate 通过。order 的首次 terminal call 把当前 `verify` 作为非终态
`preceding_finishes`，并声明 next 仍为 `verify`。该 JSON 符合结构 schema，但违反“完成节点不能再绑定自身”的
状态机硬规则；Runtime 忠实拒绝，Agent 下一请求删除 preceding finish 后成功。该问题是新的 terminal 工具
affordance 缺口，不是反馈丢失、状态提交错误或 Runtime 应自动纠正的证据。

因此本轮按用户要求暂停：不进入 R5-K/G3/H，也不继续实施 H-026 修复。

## 2. 实施结果

| Item | Before | Landed behavior | Evidence |
|---|---|---|---|
| next schema | 平铺 existing/create `anyOf` | `next.kind=existing/create` tagged union | ToolSpec + parser tests |
| terminal schema | `terminal_finish` wrapper | `terminal_node_id` 可选；省略即当前节点 | parser/scenario tests |
| init success | 仅 task/map | task/map/created node IDs/current | V2 live output |
| finish success | 仅 result/binding status | finished/result/next/current | V2 live output |
| terminal success | result/map/task | finished/result/map/task/current=null | V2 live output |
| failure | V1，状态 batch 形状不同 | V2 envelope，原始 class/code/message 不改写 | handler + live reject |
| context retention | init outer pair 可能被旧去重设计误隐藏 | identity-bearing outer pair 明确保留 | event-store + provider-context scenario |
| observer | 旧 `actions` 与旧 node echo | continuation、V2 identity、repeat finish | performance selftest/live report |

没有新增兼容分支。Action Map 的 dependency、ready、in-flight、lease、open-node terminal 等硬规则均未修改；
Runtime 没有自动 finish、bind、create、dedupe 或选择下一节点。

## 3. 验证矩阵

| Suite | Result |
|---|---:|
| `codex-tools taskspace` | 4 passed |
| `codex-core taskspace_control` | 20 passed |
| sequence aggregate focused | 1 passed |
| Action Map scenario evaluation | 9 passed |
| event-store identity outer pair | passed |
| performance observer selftest | passed |
| cost instrumentation selftest | passed |
| performance skill validation | passed |
| locked `whale` build + attestation | passed |

所有相关 Whale 自有 Rust 文件均小于 500 行。专用 tracing 事件已写入生产代码；live 结论以 canonical
call/output、provider request、Map snapshot 和固定 observer 为主证据，不依赖日志文本过滤配置。

## 4. Docker 总表

| Sample | Mode | Result | Req | Runtime tools | Provider calls | Nested | Controls | State fail | Map nodes/edges/open | Wall | Input | Cached | Uncached | Output | Req2+ cache |
|---|---|---|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|
| order | Standard | complete/solved | 8 | 14 | 14 | 0 | 0 | 0 | N/A | 46.07s | 78,805 | 73,856 | 4,949 | 4,984 | 93.27% |
| order | R5 | complete/solved | 9 | 13 | 18 | 2 | 5 | 1 | 3/0/0 | 64.96s | 107,867 | 94,208 | 13,659 | 6,761 | 86.88% |
| billing | Standard | complete/solved | 18 | 34 | 34 | 0 | 0 | 0 | N/A | 70.67s | 231,269 | 224,384 | 6,885 | 7,207 | 96.97% |
| billing | R5 | complete/solved | 15 | 21 | 26 | 1 | 5 | 0 | 5/0/0 | 61.84s | 181,985 | 171,648 | 10,337 | 6,115 | 94.23% |

R4 仍无同 revision、同 Docker contract、同 observer 口径 artifact，沿用 J7.5 的 unavailable 结论，不补造数值。

R5/Standard：order requests `1.12x`、input `1.37x`、wall `1.41x`；billing requests `0.83x`、input
`0.79x`、wall `0.88x`。两个方向相反，单次样本只证明本轮工具链正确性，不证明稳定性能优势。

## 5. Control 忠实性

| Sample | V2 success | Identity steps | Missing | Repeat committed finish | Init ID echoes | Finished/next/current echoes | Final task/open |
|---|---:|---:|---:|---:|---:|---|---|
| order | 4 | 4 | 0 | 0 | 3 | 2/2/3 | completed/0 |
| billing | 5 | 6 | 0 | 0 | 5 | 5/4/5 | completed/0 |

order live 顺序：

1. init 返回 `inspect_code/implement_fix/verify` 和 current=`inspect_code`；两个 nested shell 形成独立事件。
2. finish `inspect_code -> implement_fix` 返回 finished/result/next/current；下一请求在 `implement_fix` patch。
3. finish `implement_fix -> verify` 返回同类完整身份；下一请求在 `verify` 执行 pytest。
4. 错误 terminal `verify -> verify` 被硬规则拒绝，错误原文完整；下一请求省略 terminal target，正确完成 current。

billing 最终一个 call 原子完成两步：`fix_regressions -> verify`，随后 terminal finish `verify`。V2 输出含两个
有序 step，Map 5 个节点全部完成。该样本证明 multi-finish + terminal 能力真实采用，而非仅单测存在。

## 6. Order 逐 Request

| Req | Node / action | ms | Input | Cached | Uncached | Output |
|---:|---|---:|---:|---:|---:|---:|
| 1 | named init + 2 nested shell | 2,493 | 4,145 | 4,096 | 49 | 269 |
| 2 | 8 parallel file reads | 3,794 | 7,482 | 1,408 | 6,074 | 503 |
| 3 | initial pytest | 1,339 | 8,977 | 7,936 | 1,041 | 104 |
| 4 | diagnosis + finish inspect | 40,350 | 10,166 | 8,960 | 1,206 | 4,323 |
| 5 | one multi-file patch | 4,463 | 14,566 | 10,112 | 4,454 | 504 |
| 6 | finish implement | 1,546 | 15,163 | 14,976 | 187 | 105 |
| 7 | final pytest | 1,110 | 15,347 | 15,232 | 115 | 79 |
| 8 | invalid terminal self-loop | 4,836 | 15,742 | 15,360 | 382 | 449 |
| 9 | corrected terminal current | 3,544 | 16,279 | 16,128 | 151 | 425 |

相对旧 J7.5 order R5：requests `18 -> 9`、state failures `4 -> 1`、nodes/open `9/4 -> 3/0`、input
`220,066 -> 107,867`、wall `85.53s -> 64.96s`。该对比直接支持原反馈缺口修复，但仍是不同 Agent rollout，
不能把所有成本变化归因于单一代码改动。

## 7. Billing 逐 Request

| Req | Node / action | ms | Input | Cached | Uncached | Output |
|---:|---|---:|---:|---:|---:|---:|
| 1 | named init + README | 3,531 | 4,167 | 4,096 | 71 | 350 |
| 2 | file inventory | 3,169 | 6,620 | 2,560 | 4,060 | 269 |
| 3 | 6 source reads | 2,914 | 7,827 | 6,784 | 1,043 | 387 |
| 4 | 3 test reads | 2,153 | 8,825 | 8,192 | 633 | 201 |
| 5 | diagnosis + finish README | 16,116 | 9,547 | 8,960 | 587 | 1,809 |
| 6 | initial pytest | 1,403 | 11,440 | 11,264 | 176 | 92 |
| 7 | finish inspect_code | 3,514 | 12,862 | 11,520 | 1,342 | 320 |
| 8 | finish find_regressions | 1,575 | 13,265 | 13,056 | 209 | 102 |
| 9 | malformed patch，prepare reject | 5,002 | 13,450 | 13,312 | 138 | 554 |
| 10 | corrected 5-file patch | 4,655 | 14,069 | 13,952 | 117 | 573 |
| 11 | 4 verification reads | 2,377 | 14,751 | 14,592 | 159 | 285 |
| 12 | pytest，发现 seats 边界 | 1,392 | 15,576 | 14,976 | 600 | 94 |
| 13 | focused seats patch | 2,890 | 16,129 | 15,616 | 513 | 308 |
| 14 | final pytest | 1,399 | 16,512 | 16,384 | 128 | 92 |
| 15 | finish fix + terminal verify | 7,392 | 16,945 | 16,384 | 561 | 679 |

req9 的失败是普通 patch 语法错误：hunk context 行缺少前缀空格；反馈精确指出 line 4，req10 修正。Standard
同样有一个 prepare failure，不能归因为 TaskSpace。R5 本轮 patch max/request=1、无 multi-patch reject；Standard
本轮出现一次同 request 两 patch并被 preflight 拒绝，因此 J7 的“Agent 首次稳定只声明一个 patch”仍不是跨模式
稳定收益。

## 8. Cache 结论

没有 zero-hit、same-shape-zero 或 append-only prefix 破坏。TaskSpace 的固定一次性税来自 req1 named control 到
req2 auto tools：tool choice 和 tools shape 同时变化，order/billing req2 uncached 分别为 6,074/4,060；Standard
从 req1 就是 auto，没有该切换。

billing 的总 uncached 差额 `+3,452` 基本由 req2 相对 Standard req2 的 `+3,590` 解释，之后没有结构性 cache
失效。order 另有 req5 uncached 4,454；其前一请求生成 4,323 output，完整 message prefix 仍保留，属于大新增
Agent 内容进入下一请求后的未缓存尾部，不是上下文重排。恢复 V2 IDs 没有造成大范围 cache 破坏。

## 9. 新问题与持续观察

| Item | Classification | Evidence | Decision |
|---|---|---|---|
| terminal self-loop 可由 schema 表达 | new tool-affordance gap | order 1 state reject，随后忠实恢复 | H-026；暂停，不自动修复 |
| 多节点 Map 无 edges | persistent Agent-authored topology gap | order 3 nodes、billing 5 nodes，均 0 edges | 不判为坍缩；不让 Runtime 补边 |
| nested action 报告曾显示 0 | observer defect，fixed | canonical/cross-carrier 为 2/1 | `5f2a09d` 修复并重生成 |
| bootstrap named->auto cache tax | persistent mechanism | 两组 req2 shape transition | 后续单独评估，不与反馈修复混做 |
| dedicated tracing 文本未出现在容器日志 | log-filter exposure gap | canonical structured evidence完整 | 不影响本轮结论；后续日志专项观察 |

Map 没有坍缩：order 形成 inspect/implement/verify 三节点，billing 形成五个工作阶段；节点数量与任务复杂度匹配。
但 Agent 没声明 dependency edges，图只靠后续显式 binding 呈现顺序，作为 Map 表达质量问题继续观察。

## 10. Gate 决定

J7.6 A-D 全部完成；E 的原缺陷门禁通过，但严格 `state failure=0` 门禁因 H-026 未通过。J7.5 重算由
`11/14 -> 12/14`：Map health 恢复，R5 两样本 patch max/request=1；order 仍有1次 state failure，billing
Standard 仍产生一次 multi-patch request。J7 继续 paused。

本轮到此暂停，不执行后续修复或对抗性审查。
