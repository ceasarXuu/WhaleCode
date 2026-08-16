# repeat=3 独立异常登记

- Date: 2026-08-16
- Source: [`66-client-work-restoration-repeat3-result.md`](66-client-work-restoration-repeat3-result.md)
- Scope: 与“首次工作型序列缺少 client work”修复无因果关系的异常

## 1. 登记规则

这些表现都发生在已成功初始化 Map 并执行首个 client work 之后。它们不能计为 client work 结构前置条件失败，也不因为共享
同一次运行而合并成一个根因。已有全局问题能够容纳时不新增顶层 I 编号，只增加稳定观测标识。

## 2. 当前清单

| Stable ID | 归属 | 表现 | 频率 | Runtime 行为 | 后续行为 |
|---|---|---|---:|---|---|
| I03-ARG-SYNTAX | I03 | `taskspace_exec.arguments` 缺少合法 JSON 分隔 | 1 event / 1 run / 3 runs | 准确返回 syntax；Map/Tool 零副作用 | Agent 下一请求修正 |
| I04-FRONTIER-EARLY | I04 | 父节点未完成时直接在 Waiting 子节点执行 Tool | 1 event / 1 run / 3 runs | 列出未完成直接父节点；零副作用 | Agent 下一请求先完成父节点 |
| I04-REDUNDANT-INFLIGHT | I04 | 同批提交 Tool 时又显式把该 Ready 节点改为 `in_flight` | 2 events / 1 run / 3 runs | `TransitionInvalid`；零副作用 | Agent 下一请求删除冗余转换 |

## 3. 证据身份

| Stable ID | Run | Outer call |
|---|---:|---|
| I03-ARG-SYNTAX | 1 | `call_00_Wubd3WWV0gTdlK1zSjQY2320` |
| I04-FRONTIER-EARLY | 1 | `call_00_viJ0tShXv8cb47Th5SnD1819` |
| I04-REDUNDANT-INFLIGHT | 3 | `call_00_yk1u7DEpkZ7ZZjufDMZb4196` |
| I04-REDUNDANT-INFLIGHT | 3 | `call_00_Jwkt9ZBOzoBfB5AXxQiP4801` |

原始 rollout：

- `target/r8-client-work-restoration/repeat3-1/single-file-fast-fix/20260816-210852-165/pair-001/right/artifacts/rollout.jsonl`
- `target/r8-client-work-restoration/repeat3-3/single-file-fast-fix/20260816-210852-146/pair-001/right/artifacts/rollout.jsonl`

## 4. 边界判断

1. I03-ARG-SYNTAX 是 Agent 参数生成稳定性问题。当前反馈准确区分 syntax，没有 wrapper 注入或错误层级混淆，因此不是 I05
   反馈分类缺陷复发。
2. I04-FRONTIER-EARLY 与 I04-REDUNDANT-INFLIGHT 都涉及节点状态，但前者是选错可执行 frontier，后者是重复声明 Runtime 已按
   Tool 归属机械完成的启动转换；统计和后续根因分析不得合并。
3. 三类异常均被硬规则零副作用拦截，说明 Runtime 底线正确；这不等于 Agent 行为成本可接受。
4. 后续 repeat 扩大只统计复发率和上下文诱因，不因单次复发立即增加 Runtime 语义干预。
