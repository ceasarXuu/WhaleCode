# R8 TaskSpace Exec 多行 Patch 自愈结果

- Date: 2026-08-19
- Scope: `taskspace_exec` Function 参数中的确定性 JSON 语法修复
- API usage: 0；只使用既有 trace 和本地 Rust 测试
- Status: offline implemented and verified；尚未执行真实 Agent 复验

## 1. 直接证据

`WAR-20260819-223533-R8-BASE-CLIENT-SCOPE-R5` 的五轮运行中，两次长 `apply_patch` 被 JSON parser 拒绝。原始
Function arguments 表明这不是单个裸换行：整个 Patch 正文以原始多行字符串进入 `input`，正文中的双引号同样没有 JSON
转义。既有自愈器只覆盖单个闭合符号和单个裸换行，因此无法修复这两条真实参数。

## 2. 实现边界

本轮增加两条按顺序执行的机械修复：

1. 对 JSON 字符串内部一个或多个原始 LF 统一转义；只有修复后能够完整解码为当前 `taskspace_exec` plan 才接受。
2. 若普通 LF 转义仍失败，只对具备原生 `apply_patch` Tool 身份、`input` 字段和完整 `*** Begin Patch` / `*** End Patch`
   边界的原始 Patch 正文执行一次标准 JSON string 编码。编码后的整个 outer plan 仍必须通过同一生产 decoder。

修复后的 `ResponseItem::FunctionCall.arguments` 在写入会话历史前替换原始参数。自愈不补 Map 状态、不修改 Tool 顺序、
不推断 Agent 意图，也不执行无法通过完整 decoder 的候选。多行字符串同时伴随另一类语法错误时继续零副作用拒绝。

## 3. 验证

- `tools::taskspace_exec::self_heal::tests`：13/13 通过；覆盖单/多裸换行、真实形态多行 Patch、混合错误拒绝和既有闭合符号修复。
- `session::tests::taskspace_raw_newline_self_heal_replaces_the_item_before_history_is_recorded`：1/1 通过；确认正式上下文只保存修复后参数。
- `cargo fmt --check`：通过；仅有仓库既有 stable toolchain 配置警告。

该结果证明工程分支能够处理已观察到的参数形态，不证明目标模型后续仍会产生该形态，也不替代真实运行中的自然命中验收。
