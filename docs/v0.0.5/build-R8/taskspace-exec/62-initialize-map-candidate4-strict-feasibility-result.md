# initialize_map 候选 4：DeepSeek strict 可行性实验

- Date: 2026-08-16
- Candidate: 恢复完整首次初始化示例与原始 `$ref` schema，仅将 `taskspace_exec.strict` 从 `false` 改为 `true`
- Validation: 官方合同核对、离线 schema 遍历、缓存 final-wire 门禁
- Real Whale Agent runs: 0
- Cost: 0 CNY
- Result: rejected before Provider execution

## 1. 单变量边界

候选只切换 outer Function 的 `strict` 标志，不修改 Map 字段、合法序列、Runtime 解码或 Tool 行为。没有把可选字段改成必填，因为那会同时改变产品合同，不再是 strict 标志的单变量实验。

DeepSeek 官方 strict 合同要求：每个 object 的全部 properties 都列入 required，且 `additionalProperties` 必须为 false；strict 当前还是 Beta 能力，服务端会先校验 schema。来源：<https://api-docs.deepseek.com/zh-cn/guides/tool_calls>。

## 2. 离线结果

对实际构造路径使用的 TaskSpace Exec 测试 Catalog 做递归遍历，至少发现 7 个明确不兼容点：

| 路径 | 不兼容事实 |
|---|---|
| `$` | `additionalProperties` 不是 false |
| `$/$defs/tool_action` | `additionalProperties` 不是 false |
| `$/$defs/update_map_input/properties/node_patches/items` | `content`、`goal`、`parents`、`state` 是真实可选字段 |
| `$ / anyOf / 0` | `initialize_and_work.tools` 可省略，用于同响应 Provider Tool work |
| `$ / anyOf / 1` | `work.tools` 可省略 |
| `$ / anyOf / 3` | `update_and_work.tools` 可省略 |
| `$ / anyOf / 6` | `reopen_update_and_work.tools` 可省略 |

缓存门禁也正确发现 Tool final-wire 发生变化并保持发布阻断。由于 schema 已确定不满足 Provider strict 前置合同，继续构建、提交和真实调用不会测试 Agent 的类型生成，只会测试 Provider 是否拒绝已知非法合同。

## 3. 结论

1. “只设置 `strict=true`”不可行，候选在 Provider 执行前否决。
2. 把所有可选字段机械改成 required 会破坏 Provider-only work、Map patch 和合法序列语义，属于另一套协议设计，不在本候选范围。
3. 本候选没有进入生产代码、没有 ledger 记录、没有 API 请求或费用。
4. strict 不能作为当前 `initialize_map` string 问题的低成本修复方向。

## 4. 证据边界

- Focused test: `strict_candidate_reports_incompatible_object_schemas`（临时实验探针，结论记录后已删除）
- Observed violations: at least 7 in the focused Catalog
- Cache gate: comparable TaskSpace Tool final-wire change, release blocked
- Official contract: <https://api-docs.deepseek.com/zh-cn/guides/tool_calls>
