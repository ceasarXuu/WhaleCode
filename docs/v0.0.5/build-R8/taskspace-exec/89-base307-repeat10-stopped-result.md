# Base 3.0.7 Repeat 10 提前停止结果

- 日期：2026-08-20
- 运行账本：`WAR-20260820-060735-R8-BASE307-TYPE-R10`
- 计划：`release-dispatch-repair × map-request × repeat=10`
- 实际：第 1 轮触发业务硬失败停止条件，未启动剩余 9 轮

## 1. 执行轨迹

| Request | Agent 动作 | Runtime 结果 |
|---:|---|---|
| 1 | 一次 `initialize_and_work`，初始化四节点 Map，并在 `inspect` 执行一次 `exec_command` | 接受；Tool 成功，`inspect` 保持 `in_flight` |
| 2 | 同一 Provider 响应并列生成两次 `taskspace_exec(type=work)`；两者都绑定 `inspect`，分别读取实现和测试 | 响应级合同拒绝：`TaskSpace response contains more than one Exec call`；整批零执行，turn fatal |

Agent 没有生成 Patch，退出码为 1；公开验证与 hidden oracle 均失败。按事先声明的“业务或 Map 硬失败即停止”，本批在
`1/10` 后结束，不使用剩余预算继续采样。

## 2. 根因边界

这不是 Base `3.0.7` 的 `type` 修复回归：本轮三个 outer Exec call 全部显式携带合法 `type`。新暴露的是 outer call
基数合同缺口：

- 冻结产品不变量要求每个 Provider 响应恰好一个 outer `taskspace_exec`，多个 client 动作放入该调用的 `tools[]`。
- Agent-visible Base 只说 `taskspace_exec` 是唯一顶层 Function Tool，Tool protocol 只说选择一个 sequence `type`。
- “唯一 Tool 名称”不等于“每个响应只能调用一次”；模型把两项可并行读取生成为两个同级 outer call，是当前文本允许的
  合理误读。
- Runtime 拒绝是正确硬边界。Runtime 不应擅自合并两个 Agent call，因为合并会改变调用身份、事务和潜在 Map 顺序。

最小候选是在 Agent-visible 严格正确性合同中明确：每个响应只生成一次 `taskspace_exec`；所有 client 动作放入同一个
`tools[]`。这符合现有产品不变量，但属于新的 Base/Tool 协议候选，不能使用本批“禁止协议候选”的剩余预算直接验证。

## 3. 成本与缓存

| Runs | Requests | Input | Cached | Uncached | Output | Request 2+ cache | Agent wall | CNY |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 2 | 26,536 | 25,600 | 936 | 630 | 93.84% | 4.659s | 0.002708 |

缓存形状和 Base identity 均稳定，未发生 Tool shape 或 `tool_choice` 切换。本轮不能用于评价十轮稳定性。

## 4. Observer 修复

Runner 在业务失败之后又因 strict mode 读取空 `wait_attribution_unavailable_fields` 失败。`d74d14e9b` 已改为安全枚举
空属性集合，并在 strict mode 下增加离线回归；E3 harness guardrails 通过，当前失败 run 的 sample timing 与性能报告可
离线重建。该问题归 I07，不影响对两个 outer Exec call 的原始 Provider 证据判断。

机器可读证据：
`benchmarks/taskspace/r8/evidence/WAR-20260820-060735-R8-BASE307-TYPE-R10.json`。
