# Hosted 首轮完整示例第 2 阶段结果

- Date: 2026-08-14
- Subject: `7f7a1e7b9de5c4930aa78571ef54c3d968a54d46`
- Model: `deepseek-v4-flash`
- Planned scope: `provider-web-search-probe × map-request × repeat=3`
- Actual scope: `repeat=1`；首轮业务失败后按授权停点停止
- Result: **未通过；不进入第 3 点，不晋升缓存基线**

## 1. 单变量改动

原协议分别展示“首次初始化并执行 client Tool”和“已有 Map 上登记 Hosted 结果”。本阶段把二者合并成一个
`initialize_and_work` 示例：同一 `tools[]` 同时包含 `exec_command` 和
`web_search + execution: already_executed`，并删除第二段独立 Hosted JSON 示例。

这次替换没有扩大 Tool 声明：首请求 Tool 区从上一阶段的 26,715 bytes 降为 26,589 bytes。离线
`taskspace_exec` 测试 75/75 通过，Standard final-wire 不变；TaskSpace 的唯一缓存敏感差异是
`/tools/0/description`。

## 2. 真实运行结果

| Runs | Requests | Input | Cached | Uncached | Output | 耗时 | 估算费用 | 结果 |
|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1/3 | 1 | 12,761 | 3,968 | 8,793 | 1,702 | 34.224 s | USD 0.0017186904 | FAIL |

后两次 repeat 未执行。唯一响应返回 200，usage 完整；失败发生在响应合同核对，任何 Map 初始化和 client Tool
副作用均未提交，`provider_fact.json` 因而不存在。

## 3. Trace 事实

Agent 的 reasoning 正确说出了同响应双写意图，但实际输出为两个 `function_call`：

1. `taskspace_exec(initialize_and_work)` 内登记 `web_search + already_executed`，同时声明一个
   `exec_command`；
2. 顶层另行生成普通 `function_call(name=web_search)`，并携带 `queries` input。

第二项不是 Provider 执行完成后返回的原生 `web_search_call`。Response scope 因此观测到
`exec_call_count=1`、`hosted_tool_count=0`，并准确返回：

```text
TaskSpace response contains forbidden top-level client Tool `web_search`
```

Runtime 没有把普通 Function Call 伪装成 Hosted 结果，也没有执行 Map 或 client Tool；该拒绝符合现有硬边界。

## 4. 结论

“完整示例”这个设计在 `taskspace_exec` 自身的 description 中并不真正完整：它只能展示 Exec 内的归属登记，无法在同一
JSON 示例中展示由 Provider 产生的原生 `web_search_call`。模型据此把缺失的另一半补成了普通顶层 Function Call。
历史 `49-ls09-indivisible-pairing-partial-result.md` 已出现过同类误读；本阶段单变量改动在首请求再次触发，形成明确负向信号。

因此本阶段不能用“多跑两次”掩盖结构缺陷，也不能继续靠增加文字解释修补。候选提交保留用于可追踪对比，但不接受缓存基线、
不进入第 3 点。下一步需要用户决定：回退该候选后重审第 2 点，或放弃“在 Exec description 中提供完整双写示例”这一方向。

## 5. 证据

- Result: `benchmarks/cache-regression/results/WAR-20260814-050215-CACHE-REGRESSION-A6B34C65.json`
- Evidence: `benchmarks/cache-regression/evidence/WAR-20260814-050215-CACHE-REGRESSION-A6B34C65/`
- Local trace: `target/r8-hft-s2/run/`
- Proposal: `benchmarks/cache-regression/proposals/CBP-AC6660557AB7E186.json`
- Authorization: `benchmarks/cache-regression/authorizations/CBA-20260814-R8-HOSTED-FIRST-TURN-STAGE2-AC6660557AB7E186.json`
