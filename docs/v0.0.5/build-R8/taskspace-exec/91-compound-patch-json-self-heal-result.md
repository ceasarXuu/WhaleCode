# 复合 Patch JSON 自愈结果

- Date: 2026-08-20
- Issue: R8-I03
- Status: **确定性离线修复完成；未执行真实 Agent 复验**
- Source run: `WAR-20260820-062550-R8-BASE308-OUTER-R10`

## 原始证据

Base 3.0.8 扩大验收中的两次 JSON syntax reject 不是两个随机错误，而是同一个复合坏形状：

1. Pair 003，call `call_00_9UASnGQQN5pDkfZjyubT5948`；
2. Pair 005，call `call_00_x5nWuQlNxbrVPUxGyxnd0011`。

两次参数都在完整 `apply_patch.input` 内含一个未转义 LF，同时在 Tool action 尾部多出一个 `}`。原有自愈器只尝试
单步修复：转义 LF 后仍有多余括号，因此完整 Exec decoder 拒绝候选，自愈没有生效。

## 修复边界

自愈器现在允许按固定顺序组合两个已有机械动作：

1. 转义 JSON string 内的裸 LF；
2. 仅在解析错误邻域删除一个多余 `}`；
3. 要求裸 LF 位于唯一完整 Patch 的 `input` 内；
4. 要求结果包含且只包含一个完整 `apply_patch`，并通过当前 TaskSpace Exec Catalog 的完整解码；
5. 所有候选去重后必须只有一个确定结果。

两个多余括号、缺失 Patch 边界、非 Patch 裸换行、歧义候选或任何无法通过完整合同的结果仍保持拒绝。Runtime 不推断
Agent 意图，不修复任意两类 JSON 错误，也不放宽合法序列或 Map 状态机。

自愈继续发生在正式 ResponseItem 写入历史之前，因此执行参数和后续上下文看到的是同一份修正版；原始错误参数只通过
hash 进入观测事件，不会继续强化错误格式。

## 离线验证

- TaskSpace Exec self-heal：15/15 通过；
- 正向：裸 LF + 一个多余 action `}` 被还原为原始合法参数；
- 负向：裸 LF + 两个多余 `}` 保持拒绝；
- 既有“裸换行 + 其他语法错误”拒绝用例继续通过；
- 既有正式历史替换回归继续验证自愈后的参数在记录前替换原参数。

本改动没有启动 Whale Agent，也没有消耗 API 预算。它关闭本次两条已观察坏形状的工程缺口，不证明 Agent 不再生成
其他非法 envelope，因此 I03 保持 `verifying`。
