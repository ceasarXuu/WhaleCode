# R5-J7.7 Terminal Finish Chain 修复计划

- Date: 2026-07-13
- Status: APPROVED FOR IMPLEMENTATION
- Scope: H-026；J7.5 复验前置门

## 1. 问题与证据

J7.6 order 的最后阶段已经正确收到 `current_node_id=verify`，但 Agent 用
`preceding_finishes:[verify -> verify]` 加省略目标的 terminal finish 表达结束。输入通过 parser，随后被
状态机的“已完成节点不能重新绑定自身”硬规则忠实拒绝，下一请求删除 preceding finish 后成功。

因此根因不是上下文丢失、反馈扭曲或状态机过严，而是 `finish_then_end` 同时暴露两套节点角色：

1. `preceding_finishes[]` 使用非终态 `finish + next`；
2. `terminal_node_id` 可省略并回退到 current；
3. schema 无法表达两组字段之间的身份不等式和完整链关系。

工具参数于是允许“结构合法、生命周期非法”的 self-loop。修复必须发生在工具合同，不得让 Runtime 猜测、删除
或改写 Agent 动作。

## 2. 外部依据

- [DeepSeek Tool Calls](https://api-docs.deepseek.com/guides/tool_calls/)：工具输入以 JSON Schema 暴露；strict
  模式只支持有限 schema 子集，array 的 `minItems/maxItems` 明确不受支持，不能依赖复杂跨字段约束修复合同。
- [JSON Schema array reference](https://json-schema.org/understanding-json-schema/reference/array)：数组顺序可作为
  明确协议语义；`uniqueItems` 可表达值唯一，但不能表达运行时 current identity 或跨对象生命周期关系。
- [MCP Tools specification](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)：tool input schema
  是工具能力与参数的正式合同；结构化结果应与声明的输出合同一致。

结论：用更小的单一有序结构直接表达生命周期链，不继续叠加跨字段校验或提示词。

## 3. 冻结合同

`finish_then_end` 原子替换为：

```json
{
  "action": "finish_then_end",
  "finish_node_ids": ["fix_regressions", "verify"],
  "final_candidate": "Agent-authored final answer"
}
```

机械语义：

1. `finish_node_ids` 由 Agent 显式声明，顺序即完成顺序；
2. 每个非末尾节点完成后绑定数组中的下一个节点；
3. 最后一个节点执行 terminal finish，并原样发布 `final_candidate`；
4. 单节点结束使用 `finish_node_ids:[current]`；
5. 数组必须非空、节点 ID 非空且不重复；
6. Runtime 不选节点、不排序、不补节点、不推断目标、不改写 final candidate；
7. 旧 `preceding_finishes`、`terminal_node_id` 和更早的 `terminal_finish` 均不可解析，不保留兼容分支。

数组级唯一性仍在 typed parser 做机械校验，因为目标 DeepSeek strict schema 不支持足够的 array 关键词；这不是
语义决策。状态机继续保留 dependency、ready、owner、lease、open node 和 terminal 等全部硬规则。

## 4. 实施阶段

### J7.7-A：schema 与 typed parser

- ToolSpec 只暴露 `finish_node_ids[] + final_candidate`。
- typed args 使用 `Vec<String>`，拒绝空数组、空 ID 和重复 ID。
- schema fixture 证明旧形状不可表达；parser fixture 证明旧形状不可解析。

退出：input contract 单一，无 compatibility path。

### J7.7-B：原子 terminal chain

- Action Map 在 clone 上按 Agent 声明链执行全部非终态和终态迁移。
- 全链成功后一次提交；任一步硬状态失败时原 Map 不变。
- Session 一次发出完整有序事件；handler 输出 V2 有序 step identity。

退出：单节点、双节点、三节点成功；duplicate/invalid/open-map 失败零部分提交。

### J7.7-C：反馈、日志与 observer

- 成功输出逐步返回 `finished_node_id/result_id/next/current`，terminal 返回 `current=null`。
- 增加 chain declared/committed/rejected 的机械日志，仅记录 call ID、step count、reason code。
- observer 识别 `finish_node_ids`，正确计算 finish count、identity echo 和 committed repeat。

退出：identity coverage 100%，observer selftest 通过。

### J7.7-D：工程回归与构建

- `codex-tools taskspace`、`codex-core taskspace_control`、sequence、Action Map scenario、event store。
- performance observer、cost instrumentation、skill validation。
- locked `whale` build 与 binary attestation。

退出：相关测试和构建全部通过，git clean。

### J7.7-E：J7.5 Docker 复验

- 同一 binary、Docker substrate 和 observer 跑 `multi-file-order-pipeline`、
  `subscription-billing-repair` 的 Standard/R5 pair。
- 逐 request 记录动作、时间、input/cached/uncached/output、control output、Map 和 patch manifest。
- R4 仅接受同 contract artifact；不可用时明确记录，不补造结果。

退出门禁：

```text
两组 Standard/R5 complete + external solved。
R5 protocol/state failure = 0。
terminal chain duplicate = 0。
success identity missing = 0；committed repeat finish = 0。
R5 Map open = 0；task/map completed。
request-wide multi-patch executed = 0；R5 patch max/request = 1。
无工具反馈、权限、沙箱、cache prefix 或 correctness 回退。
```

单次 Standard 偶发声明 multi-patch 时必须与“执行安全门禁”和“Agent 首次采用率”分账，不允许把 Runtime 拒绝
伪装成 Agent 首次只声明一个 patch。

## 5. 工程收益

| 目标 | 基线 | 目标值 | 验证 |
|---|---:|---:|---|
| terminal self-loop state reject | order 1 | 0 | Docker control trace |
| terminal recovery request | order 1 | 0 | request trace |
| terminal chain partial commit | 当前结构可能逐步提交 | 0 | failure snapshot/hash |
| success identity missing | 0 | 0 | V2 observer |
| Runtime semantic decision | 0 | 0 | code/schema audit |

## 6. 暂停与回退

- provider 无法稳定生成新形状：记录 tool usability 证据并暂停，不恢复旧形状。
- chain 任一步失败后 Map 有部分变化：回退 J7.7-B，不进入 live run。
- live state/protocol failure 非零：保留原始 trace，新增 CoE evidence，不进入 R5-K。
- J7.5 只有全部 correctness gate 通过才关闭；成本只报告，不以降成本替代正确性。

