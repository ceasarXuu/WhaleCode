# Phase B5 CP-11：Hosted 分类与逐项核对

- Date: 2026-08-10
- Scope: TaskSpace Exec Catalog、Provider response scope、Hosted preflight/settlement
- Status: verified offline

## 1. 问题

Hosted Tool 的输入声明与输出识别原本各自维护 Web Search/Image Generation 的类型判断。两处当前结果一致，但新增或修改
Hosted 类型时可能单边漂移，使 Agent 可声明的类型与 Runtime 实际识别的 Provider output 不一致。

逐项核对本身已经按 Provider output index 排序，并将 Agent 声明的一个或多个 owner node 写入各 Node actions；需要补齐的
是同一事实来源和完整故障矩阵，而不是新增归属推断。

## 2. 修复

1. 新增最小 `HostedToolKind`，统一从原生 `ToolSpec` 和原生 `ResponseItem` 识别 Hosted 类型及公共名称；
2. `HostedOutputFact` 与终态 outcome 转换归入同一事实边界，Catalog 和 response scope 不再分别手写 Web/Image 名称；
3. 保留既有 response-local 核对：按真实 output index 排序，校验数量、Tool 类型、Provider ID、owner node 和重复项；
4. Provider 的 `failed`/`cancelled` 原样成为 action outcome，不触发节点完成、失败或其他状态变化；
5. 不增加结果存储、默认 Root、猜测归属、重执行或语义性修复。

Agent 对 Hosted output 的节点归属仍由 `hosted_bindings[].node_ids` 明确声明。Runtime 只把该声明与 Provider 已发生事实逐项机械
比对；一个 output 可以属于多个 Work node。

## 3. 验证

- `cargo test -p codex-core taskspace_exec --lib`：69 PASS；
- Catalog 与 response item 对 Web/Image 使用相同 Hosted identity；
- Provider output 乱序输入按真实 index 还原，多节点归属保持；
- 数量不匹配、Tool 错配、空/重复 Provider ID、重复 output index、空/重复/未知/边界 node 均 fail closed；
- failed/cancelled outcome 被忠实保留；Hosted 失败写入 Node action 后，节点状态仍由 Agent 的 Map 操作决定；
- Standard Tool 路径、Provider Hosted 执行和 TaskSpace client dispatch 均未改变。
- zero-base gate PASS；cache gate 的免费 final-wire 比较 PASS，候选敏感面指纹为
  `43cee2c0c284bb257dd4529f3be4e22416d0c71f23ee1ea7388518e3330bd659`，发布继续阻断等待获批真实回归。

## 4. 后续

CP-12 只加固活动协议权威和 final-wire/cache 门禁，不再扩展 Hosted 语义。真实 Provider 对 Hosted 调用的行为证据留在完成
CP-13 后的获批验证中。
