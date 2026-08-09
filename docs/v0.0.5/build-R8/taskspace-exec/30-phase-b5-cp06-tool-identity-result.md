# Phase B5 CP-06：结构化 Tool identity 落地结果

- Date: 2026-08-10
- Scope: TaskSpace Exec Catalog、Agent-visible call schema、decoder、preflight 与 native dispatch
- Status: verified offline

## 1. 产品合同

普通 Function、Freeform 和 Map operation 保持 `tool: "name"`。Namespace Function 使用两个独立字段：

```json
{
  "tool": "lookup",
  "namespace": "mcp__sample__",
  "node_id": "inspect",
  "arguments": {}
}
```

`namespace` 与 `tool` 都由最终 Catalog 生成精确 enum。旧的 `mcp__sample__lookup` 扁平 alias 已从 TaskSpace
schema、decoder 和 dispatch lookup 删除，不提供兼容 reader。

## 2. 实现边界

1. Catalog 以原生 `ToolName { namespace, name }` 为唯一 client capability key；
2. decoder 从结构字段直接构造 `ToolName`，Map operation 只占用无 namespace 的名字；
3. preflight 用同一个 `ToolName` 回查 Catalog，native dispatch 原样写回 `ResponseItem.namespace/name`；
4. 同名普通 Tool、多个 Namespace child，以及与 Map operation 同名的 Namespace child 可以共存；
5. Code Mode 继续使用其自身 JS 调用别名，不复用 TaskSpace wire，也未被本次修改；
6. 日志和 Map action 中的字符串只作展示，不参与执行身份解析。

## 3. 验证

- TaskSpace Exec：62 tests PASS；
- Tool capability projection：7 tests PASS；
- `ToolName` 结构化序列化 round-trip PASS；
- 同 leaf 的 plain / alpha namespace / beta namespace 可同时解码；
- namespaced `read_map` 不会被误判为 Map operation；
- 旧扁平 alias、缺失 namespace、错误 namespace 和 `namespace: null` 均被拒绝；
- 重复的同一二元身份在 Catalog 构建时 fail closed。

本单元改变 TaskSpace outer declaration，因此能力身份版本升级为 v2，并必须通过缓存敏感面门禁。真实 Provider
遵循度与成本不在 CP-06 离线结论中，留到 CP-13 之后按已批准预算执行。

## 4. 后续依赖

Deferred Tool Search 在 TaskSpace 内层执行后，Provider 不会自动安装其返回能力。后续先由 CP-09 保留 typed nested
result，再由 CP-05 从自然上下文中的该结果机械恢复已加载 capability；不得回退到首轮展开全部 deferred schema，亦不得
增加隐藏 session ledger。
