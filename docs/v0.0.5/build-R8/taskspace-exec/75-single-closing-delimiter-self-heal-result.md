# 单闭合符号自愈边界补全结果

- Date: 2026-08-17
- Scope: `taskspace_exec.arguments` 落账前机械规范化
- Result: 单个多余 `}` / `]` 已纳入；没有扩张到通用 JSON 猜测
- Provider runs: 0

## 1. 观测 Case

历史调用 `call_00_r1gFozV8SSGkRFy7xDK20229` 在 Tool action 末尾生成了相邻的两个 `}`：

```text
..."*** End Patch"}}]}
```

删除其中任意一个位置都会得到同一个不同的参数字符串。该字符串：

1. 通过严格 JSON 与当前形状的 Exec plan 解码；
2. SHA-256 为 `e9fa45fa32abdcf2d0da8b8c9b96c03077226e1f62c7fc056cec406dc3684db8`；
3. 与 Agent 下一请求 `call_00_xdtlbkZzteFoAmUJmtJv6411` 的成功重试逐字一致。

因此根因是单个多余闭合符号，不需要 Runtime 猜测 Tool、节点、参数或动作语义。

## 2. 实施规则

自愈器现在在同一个有界候选集中执行两类编辑：

- 插入一个 `}` 或 `]`；
- 删除一个现有的 `}` 或 `]`。

所有候选都必须位于 JSON parser 首个错误附近或正文末尾、不得位于 JSON 字符串内部，并通过当前 request Catalog 的完整
Exec plan decode。最终只有一个不同的合法候选时才改写；零个或多个候选一律保留原文并走正常拒绝路径。相邻同字符即使有
两个可删除位置，只要产生的是同一个修复字符串，仍属于一个候选。

修复发生在 `ResponseItem` 进入 response scope、history、rollout 和 dispatch 之前。正式上下文只保留修正版；审计事件只记录
`repair_operation`、符号、字节位置和修复前后摘要，不记录参数正文。

## 3. 历史异常复扫

扫描本机 `target/r8*/**/rollout.jsonl` 中 13 次 `taskspace_exec` JSON syntax reject：

| 分类 | 次数 | 处理结论 |
|---|---:|---|
| 单字符删除可形成唯一 JSON 候选 | 3 | 2 次多余 `}`、1 次多余 `]`；纳入本次规则，最终仍须当前 Catalog 解码 |
| 插入闭合符号可形成 JSON 候选 | 8 | 已属于原 SR-01；多个 JSON 候选时仍由完整 Catalog 唯一性决定，否则拒绝 |
| 复合字符串转义损坏 | 1 | 同时包含裸换行和未转义引号，无法无歧义确定字符串边界；不自愈 |
| 多个结构错误 | 1 | 删除一个闭合符号后仍非法；不自愈 |

本轮没有发现应新增的第二类安全修复。逗号、引号、控制字符、字段、值、动作、节点和顺序仍在明确禁区；即使某种文本编辑能让
JSON parser 通过，也不能在没有唯一完整 Exec plan 的情况下接受。

## 4. 离线验收

- 自愈单测：9 passed，覆盖缺/多 `}`、缺/多 `]`、中文 byte column、歧义、多错误、非 Exec 和合法输入零改写；
- Session hook：修复后的同一个 `ResponseItem` 在写入 history 前被替换；
- 无真实 Whale Agent run，无 API 成本。

该变更只减少明确序列化小错造成的一次拒绝与重试，不改变 TaskSpace 合法序列、DAG、Map 生命周期、Tool 权限或 Agent 决策边界。
