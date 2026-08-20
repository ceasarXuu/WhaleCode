# Phase B5 CP-02：Tool identity 证据与决策结果

- Date: 2026-08-10
- Scope: `ToolName`、Function / Freeform / Namespace 的 Agent 可见身份
- Status: verified；Namespace wire 已确认并由 CP-06 落实
- Production behavior: CP-02 只提供证据，生产变更见 CP-06

## 1. 结论

Codex Runtime 已用 `ToolName { namespace: Option<String>, name: String }` 保存原生 Tool 身份。当前
`nested_tool_public_name()` 只为 Code Mode 和 TaskSpace Exec 生成扁平展示名，它不是可逆身份编码。

当前构造器不限制 `namespace` 或 `name` 的字符，因此以下三个不同身份会得到同一个字符串：

```text
(namespace="alpha",      name="beta_gamma") -> alpha_beta_gamma
(namespace="alpha_beta", name="gamma")      -> alpha_beta_gamma
(namespace=null,          name="alpha_beta_gamma") -> alpha_beta_gamma
```

当前“namespace 末尾已有下划线或 tool name 以下划线开头时不补分隔符”的启发式也会让
`("alpha_", "beta")` 与 `("alpha", "_beta")` 相撞。Catalog 的重复检测只能拒绝整个能力面，不能恢复
原生身份，也不能让所有合法 Tool 共存。

## 2. 已验证事实

1. Function 与 Freeform 的原生身份是 `namespace = null` 加原名，不需要增加新字段。
2. Namespace 在 Provider ToolSpec 中本来就是 `namespace.name + child.name` 两个字段，Router handler 也按结构化
   `ToolName` 查找；扁平化只发生在嵌套 Tool 的 Agent 可见别名层。
3. `ToolName` 直接 JSON 序列化可以对包含下划线、斜线、冒号等字符的普通和 Namespace 身份无损往返。
4. TaskSpace Catalog 已持有原生 `ToolName`，因此 Runtime 不需要从描述文字或 Tool 输出猜测身份。

证据测试：

- `codex-tools`: `current_nested_public_name_is_not_a_reversible_identity`
- `codex-tools`: `current_boundary_heuristic_also_collides`
- `codex-protocol`: `serde_round_trips_plain_and_namespaced_identity_without_flattening`

## 3. 最小候选

推荐仅调整 Namespace call variant：

```json
{
  "tool": "lookup",
  "namespace": "mcp__sample__",
  "node_id": "inspect",
  "arguments": {"value": "x"}
}
```

普通 Function、Freeform 和 Map operation 继续保持现有 `tool: "name"` 结构。Namespace variant 的 `tool` 与
`namespace` 都由 Catalog 生成精确 enum；decoder 直接构造原生 `ToolName::namespaced(namespace, name)`。

该候选的收益：

- 不发明分隔符、转义、长度前缀或 JS identifier normalization；
- 只改变 Namespace variant，不增加普通 Tool 的 token 和结构成本；
- 与 Provider、Registry、Router 的原生二元身份一致；
- 多个 Namespace 可以拥有同名 child，普通 Tool 也可以拥有相同显示名。

## 4. 未采用候选

| 候选 | 未采用原因 |
|---|---|
| 继续扁平化并在碰撞时拒绝 Catalog | 合法 Tool 面会因命名偶合整体不可用，且身份仍不可逆 |
| 自定义可逆标量编码 | 必须新增保留前缀、转义和解析协议；名称可读性下降，并把原生二元身份改造成 TaskSpace 私有编码 |
| 根据当前 Catalog 为碰撞项追加 hash | 新增 Tool 会改变已有别名和缓存前缀；身份依赖集合而非 Tool 自身 |
| 所有 Tool 都改成 `tool: {namespace, name}` | 可逆但扩大普通 Function、Freeform 和 Map operation 的 Agent 可见结构及 token 成本 |

## 5. 决策结果

用户于 2026-08-10 确认采用上述结构化 Namespace wire：

1. CP-06 直接破坏性替换旧扁平 alias；
2. 不保留兼容 reader、旧字段或双协议；
3. Code Mode 的 JS 别名属于独立传输协议，不因 TaskSpace wire 改动；
4. TaskSpace Catalog、decoder 和 dispatch lookup 必须始终使用原生二元 `ToolName`。
