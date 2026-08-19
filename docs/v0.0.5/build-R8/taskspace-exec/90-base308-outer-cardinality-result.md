# Base 3.0.8 外层 Exec 基数验收

- Date: 2026-08-20
- Issue: R8-I03；同时暴露 I07 runner 选择缺口
- Candidate: `1c4e6d9ba`
- Ledger: `WAR-20260820-062550-R8-BASE308-OUTER-R10`
- Result: **目标缺陷在 7 次 TaskSpace / 59 个响应中未复发；批次因 runner 选择错误与单轮请求上限未完成计划的 repeat=10**

## 修复边界

Base `3.0.8` 与 `taskspace_exec` Tool description 共同明确：一个执行 Map 或 client work 的响应只能生成一个外层
`taskspace_exec`，该响应的所有 client actions 放入同一个 `tools[]`。Runtime 仍拒绝多个 outer Exec，不合并、不重排，
也不改变调用身份或事务语义。

离线验收通过：Base 6 项、Catalog 1 项、`cargo fmt --check`、`git diff --check`、zero-base gate 和缓存敏感面免费
final-wire gate。

## 真实结果

| 模式 | 实际运行 | Runner 完整成功 | 请求 | Input | Cached | Uncached | Output | Agent wall | 估算费用 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| TaskSpace map-request | 7 | 6 | 59 | 1,034,123 | 917,504 | 116,619 | 32,444 | 241.71s | CNY 0.19985708 |
| Standard（非计划） | 6 | 6 | 45 | 617,616 | 601,984 | 15,632 | 19,173 | 153.25s | CNY 0.06601768 |
| 合计 | 13 | 12 | 104 | 1,651,739 | 1,519,488 | 132,251 | 51,617 | 394.95s | CNY 0.26587476 |

TaskSpace request 2+ 缓存命中为 `88.69%`；Standard 为 `97.25%`。七次 TaskSpace 都形成 1 个 Map、5 个节点、
4 条边并最终闭合 Root。第 7 次已正确修改代码、9/9 测试通过并在第 10 个请求执行 `finish_map`；随后生成自然语言
final 所需的新请求触及单轮 10-request 上限，CLI 以 1 退出，因此 runner 将该轮标为 interrupted，而不是 Map 或代码失败。

## 目标判断

- 59 个 TaskSpace Provider 响应中，53 个包含一个 outer Exec，6 个为最终自然回复；`exec_call_count > 1` 为 **0**。
- 多 outer Exec 拒绝为 **0**，上一版首轮出现的 sibling Exec 没有复发。
- 因此“一响应一个 outer Exec”的修复在本批观测中获得支持，Runtime 无需增加自动合并。
- 只完成 7 次 TaskSpace，不能写成 repeat=10 完成，也不能据此关闭整个 I03。

## 独立异常

本轮仍有 2 次可恢复 JSON syntax reject、1 次顶层 `exec_command` 逃逸拒绝、1 次缺失 `type` 合同拒绝，以及 1 次
普通 `apply_patch` 格式失败后纠正。这些异常都不是多个 outer Exec，但证明 I03 的参数和入口稳定性仍需继续收敛。

## 执行偏差

计划误将 `-RunSide right` 理解为“只运行 TaskSpace”。它实际选择物理右侧，而 benchmark 每个 pair 交替左右逻辑模式，
所以偶数 pair 真实运行了 6 次 Standard。最终实际为 13 samples / 104 requests / 1,651,739 input，超过账本声明的
10 samples / 100 requests / 1.6M input；费用 CNY 0.26587476 仍低于 CNY 0.45 上限。发现后没有补跑剩余 3 次 TaskSpace。

后续真实单模式预算不得再用物理 side 选择模拟逻辑 mode 过滤；在 I07 修复前，应使用能直接声明逻辑模式或明确任务列表的
runner 入口，并在启动前做零 API dry-run 展开校验。

原始证据：[`../../../../benchmarks/taskspace/r8/evidence/WAR-20260820-062550-R8-BASE308-OUTER-R10.json`](../../../../benchmarks/taskspace/r8/evidence/WAR-20260820-062550-R8-BASE308-OUTER-R10.json)。
