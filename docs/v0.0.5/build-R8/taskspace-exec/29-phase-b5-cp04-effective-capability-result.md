# Phase B5 CP-04：Effective capability 单一事实视图

- Date: 2026-08-10
- Scope: Tool Registry plan、Configured ToolSpec、Router、TaskSpace Catalog
- Status: verified offline

## 1. 变更

每个 `ConfiguredToolSpec` 现在保存同一注册条目的两个机械视图：

1. `spec`：Provider 实际使用的声明；Code Mode 开启时可以包含其专属描述增强；
2. `native_spec`：进入 Code Mode 描述增强前的原生 Tool 合同，名称、输入 schema、输出 schema、deferred 标记和
   Hosted 类型均保持不变。

`ToolRegistryPlan::push_spec()` 在唯一构建点同时生成这两个视图，`ToolRegistryBuilder` 原样接收整个注册条目。
没有第二次构建 plan，没有第二个 handler registry，也没有按名称猜测或字符串清洗。

`ToolRouter::taskspace_capability_specs()` 只机械投影每个注册条目的 `native_spec`。TaskSpace Catalog 从该视图生成：

- outer declaration；
- decoder lookup；
- capability identity；
- client dispatch 保存的原生 `ToolName`；
- Hosted 分类输入。

## 2. 行为结果

| 场景 | Standard / Code Mode | TaskSpace |
|---|---|---|
| 普通模式 | Provider-visible ToolSpec 不变 | 内层合同与原生 ToolSpec 相同 |
| Code Mode | 顶层 `exec/wait` 与增强后的说明不变 | 排除递归 `exec/wait`，普通内层 Tool 不携带 JS Exec 调用语法 |
| Code Mode Only | Standard 仍只暴露 `exec/wait` | TaskSpace 仍能使用同一注册条目中的普通 client Tool |
| Hosted Tool | Standard 声明不变 | 仍只作为顶层 Hosted Tool，Catalog 只记录类型 |
| parallel safety | 继续使用同一 `ConfiguredToolSpec.supports_parallel_tool_calls` 与原 Router | 未新建并行规则 |

## 3. 约束

- `native_spec` 不是第二套 Tool schema，也不能被独立注册或修改；它与 Provider spec 同属一个
  `ConfiguredToolSpec`；
- TaskSpace 不从 `model_visible_specs()` 反推 client 能力，因为 Code Mode Only 会隐藏应由 Exec 承载的普通 Tool；
- TaskSpace 不从 raw Provider `specs()` 复制描述，因为该描述可能已被 Code Mode 增强；
- deferred Tool 的首轮展开与搜索后安装仍由 CP-05 处理，本单元只保证其原始标记和合同不丢失；
- Namespace identity 的 Agent-visible wire 仍等待 CP-02 决策，不在本单元修改。

## 4. 验证

- `cargo check -p codex-tools`
- `cargo check -p codex-core`
- `taskspace_cp01_records_code_mode_only_surface_difference`
- `code_mode_augments_builtin_tool_descriptions_with_typed_sample`
- `configured_tool_spec_name_delegates_to_tool_spec`
- TaskSpace Catalog、Router、CP-01 和缓存门禁总回归（见提交门禁输出）

关键断言同时证明：Provider `spec` 仍含 Code Mode 的 typed declaration，而同一注册条目的 `native_spec` 保持原始说明；
TaskSpace 最终 declaration 包含 `exec_command/write_stdin`，但不再包含 `exec tool declaration`。

缓存门禁对 staged final wire 的免费比较通过，候选指纹为
`93c5777d9e4464c5fb0c79971150fcfaa5d4522380395ea1c554bac20109712b`。该变更只影响 TaskSpace outer declaration；
基线未晋升，发布继续阻断，等待 CP-13 和已批准预算内的真实回归。
