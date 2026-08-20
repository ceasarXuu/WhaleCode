# IC-03 Tool Declaration 拆分结果

- Status: Complete
- Date: 2026-08-17
- Scope: Provider wire 观测，不改变 Tool schema、Tool 暴露集合、序列化或 Agent 行为

## 实现

`tools` 总 section 保持不变，并新增九类精确 byte 明细：

- `tools_envelope`：顶层 `tools` 字段、数组括号和分隔符；
- `taskspace_protocol`：`taskspace_exec.description`；
- `taskspace_client_catalog`：`parameters.$defs.tool_action` 中内嵌的原生 client Tool 合同；
- `taskspace_map_schema`：除 `tool_action` 外的 Map 操作定义；
- `taskspace_sequence_schema`：`parameters` 中合法序列的顶层结构；
- `taskspace_metadata`：TaskSpace Tool 名称、对象/`parameters`/`$defs` 外壳和分隔符；
- `native_client_tool`：Standard 顶层原生 client Function Tool；
- `provider_hosted_tool`：Provider 原生 ToolSpec；
- `other_tool`：无法由原生结构识别的 ToolSpec。

TaskSpace 内部分项按 JSON 字段边界机械计数，外壳与分隔字节显式归入 metadata/envelope。分类不比较说明文字、不推断语义，
也不把 Provider-hosted Tool 的内部 action 拆成多个 Tool。

## 完整性

- 每个顶层 ToolSpec 恰好进入 TaskSpace 分解、native client、Provider-hosted 或 other 之一。
- 九类分项 bytes 之和必须等于 `tools.bytes`；Debug 构建中不闭合即断言失败。
- 观测只输出 count、bytes、估算 token 和 hash，不输出 Tool description 或 schema 原文。
- `taskspace_client_catalog` 与 Standard 顶层 `native_client_tool` 是不同 wire 位置的面积，不据此自动认定语义重复。

## 验证

| 验证 | 结果 |
|---|---|
| TaskSpace/native/Hosted/unknown 混合 Tool fixture | passed |
| Tool breakdown 与 tools section bytes | exact |
| `cargo test -p codex-core provider_wire_sections --locked` | 14 passed |
| final-wire 缓存门禁 | passed；payload unchanged |
| 真实 Whale Agent run | 未执行 |

## 工程约束

观测代码已拆分为 section、history、Tool 和 hash 四个小模块；最大生产文件 477 行。没有把分析逻辑塞回 TaskSpace Runtime，
也没有增加 Agent-visible 协议。
