# R7 TaskSpace 核心工作协议 v1.0.1 结果

- 日期：2026-07-19
- 协议版本：`1.0.1`
- 规则哈希：`8ffae2bc82bcc3b6ce2494f47ab4014aba488994788d484e405dccc1c63484db`
- 实现提交：`deacc3405`
- 状态：核心协议保持启用；当前仍为实验候选，不冻结为最终文本
- 机器结果：`benchmarks/taskspace/r7/working-protocol-v1.0.1-result.json`

## 1. 结论

内置核心工作协议的架构方向成立：它以 TaskSpace-only 静态 developer 前缀交付，三种 projection policy
共享，Standard 零注入，不进入自然历史，不携带 Map 状态，也不让 Runtime 代替 Agent 做任务决策。

本轮 simple、complex 的 Standard 与 TaskSpace 均 solved。两个 TaskSpace run 都把 `initialize_map` 作为
首个工具，主动闭合全部 Work、Root、Finish 并提交 `finish_end`；`map-request` 仍为零 automatic projection、
零 `read_map`，因此“必须持续暴露当前 projection 才能维持生命周期”不成立。

但 `v1.0.1` 新增的 same-response lifecycle batching 指令没有产生目标行为。两组都仍有 3 次
standalone nonterminal transition，multiple control response 为 0。继续强化提示词不是正确修复方向；
`taskspace_control` schema 将 `complete` 建模为无 continuation 的独立 variant，虽然执行器支持有序 sibling
calls，模型仍自然倾向等待状态结果。这应进入后续共享 tool contract 设计。

## 2. 同期 Docker 对照

每个样本 1 次，仅作为工程诊断。

| 样本 / 指标 | Current Standard | TaskSpace map-request + v1.0.1 |
|---|---:|---:|
| simple 结果 | solved | solved |
| simple provider request | 7 | 9 |
| simple ordinary / control | 9 / 0 | 9 / 7 |
| simple wall time | 18.33s | 25.46s |
| simple input / uncached input | 50,420 / 2,036 | 97,684 / 10,900 |
| simple request 2+ cache hit | 95.69% | 96.47% |
| complex 结果 | solved | solved |
| complex provider request | 12 | 13 |
| complex ordinary / control | 20 / 0 | 20 / 7 |
| complex wall time | 46.47s | 55.86s |
| complex input / uncached input | 120,164 / 6,372 | 189,049 / 9,081 |
| complex request 2+ cache hit | 94.47% | 95.05% |

本轮 TaskSpace request 仅比 Standard 多 `2` 和 `1`，且 ordinary tool 数完全相同。与 `v1.0.0` 的
24/21 requests 相比明显下降，但旧轮包含大量 patch 失败和恢复，因此不能把下降归因于协议版本。

协议在 TaskSpace 的 22/22 个请求中版本、哈希完全一致，固定在 wire message index 1，wire role 为
`system`；Standard 19/19 个请求协议 count 为 0。message prefix preservation 全部为 100%，静态前缀没有
破坏后续缓存。

## 3. 固定成本

`v1.0.1` 协议约 431 estimated tokens/request，高于 `v1.0.0` 的 396。原因是 same-response、连续
revision 和 terminal batching 的解释增加了文本；“本轮做了压缩”这一预期没有实现。

该成本位于固定缓存前缀，不能与 uncached input 等价，但每次请求仍会计入总 input。后续若压缩协议，必须
新建版本并单独比较 simple、complex、Standard 与当前基线，不得把文本压缩和 tool contract 改造混在一轮。

## 4. 后续边界

1. 保留 `v1.0.1` 作为当前实验协议，协议身份和效果继续进入固定 benchmark 产物；
2. 不再通过增加提示词强度追求 lifecycle batching；
3. 后续设计共享的 Agent-declared `complete + next bind/finish` 工具形态，Runtime 只机械校验和执行；
4. 该工具改造必须三种 projection policy 共用，不得产生 policy 分叉；
5. 协议文案压缩作为另一个独立版本实验。
