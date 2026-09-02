# Problem P-001: Codex 0.151 TaskSpace / Extension 组合链回归

- Status: fixed
- Created: 2026-09-01 05:00
- Updated: 2026-09-01 05:45
- Objective: 在不改变 TaskSpace 状态权威和用户入口的前提下，使既有 Extension 实现兼容 Codex 0.151，并恢复 app-server schema 与组合链回归。
- Symptoms:
  - `codex-taskspace-extension` 无法实现 0.151 引入的高阶生命周期 `ToolExecutor<ToolCall<'call>>` 合同。
  - TaskSpace 测试构造的 `ToolCall` 缺少 0.151 新增的 `source` 字段。
  - app-server protocol 的 TypeScript 和预计算 export fixtures 落后于当前协议源。
  - core tool-search 测试在 Whale 默认 DeepSeek provider 下错误沿用了上游默认 OpenAI namespace-tools 前提。
- Expected behavior: TaskSpace 继续使用单一 relational store；Extension tool 可跨调用生命周期执行；schema 可重现；provider 能力测试显式声明 fixture，不改变生产默认值。
- Actual behavior: 四项均为 0.151 工程接口或测试 fixture 迁移遗漏，修复后定向矩阵通过。
- Impact: 修复前 W5 无法形成可验证闭环；未发现用户可见产品语义变化。
- Environment: Linux；分支 `whalecode-codex`；vendor `rust-v0.151.0`。
- Fix criteria: TaskSpace extension/state/core/app-server/TUI 定向测试通过，schema fixtures 可复现，测试不再依赖隐式 provider 默认值。
- Current conclusion: 兼容缺口已闭合；旧 `shell_command` 快照差异是 0.151 将旧别名归一到 `UnifiedExec` 的预期变化，不恢复重复工具。

## Hypothesis H-001: Extension executor 生命周期合同落后

- Status: confirmed
- Claim: TaskSpace executor 仍按旧的单一生命周期 trait object 实现，无法满足 0.151 的 `for<'call>` executor 合同。
- Evidence gate: satisfied
- Evidence: 编译器在 `tool.rs` / `extension.rs` 报告生命周期不充分；改为 `impl<'call>`、显式 `handle<'a>` 与 `'call: 'a` 后 `codex-taskspace-extension` 41/41 通过。
- Repair: 仅迁移既有 adapter 的类型和生命周期边界，不改变执行顺序或状态存储。

## Hypothesis H-002: ToolCall fixture 落后于 source 字段

- Status: confirmed
- Claim: 四个 TaskSpace 测试 fixture 未提供 0.151 新增的 `ToolCallSource`。
- Evidence gate: satisfied
- Evidence: 补充 `ToolCallSource::Direct` 后 extension 测试通过；生产调用路径未改。
- Repair: 更新 test-only fixture。

## Hypothesis H-003: schema fixtures 未随协议源再生成

- Status: confirmed
- Claim: app-server protocol 源已包含 0.151/TaskSpace RPC，但预生成 TypeScript 与 compressed exports 仍是旧版本。
- Evidence gate: satisfied
- Evidence: 初始 schema fixture 测试 3 项失败；运行仓库 `write-app-server-schema` 生成流程后 6/6 通过、1 ignored。
- Repair: 提交规范生成物，不手工编辑 schema。

## Hypothesis H-004: tool-search 测试隐含依赖上游默认 OpenAI provider

- Status: confirmed
- Claim: Whale 将默认 provider 保持为 DeepSeek 后，测试虽打开 model 的 search capability，却未打开 provider 的 namespace-tools capability，造成五个测试误报。
- Evidence gate: satisfied
- Evidence: 失败列表均要求 `tool_search` 或 namespace MCP 可见；测试显式选择 OpenAI provider fixture 后 tool-search 18/18 通过。
- Repair: 只修测试前置条件；不为 DeepSeek 开启未声明的 namespace-tools 能力，也不改变默认 provider。

## Ruled out

- 未发现第二套 TaskSpace 状态权威。
- 未发现 fork/reload/finalization 的功能回归。
- 不恢复 0.151 已移除的重复 `shell_command` wire tool；配置别名仍反序列化为 `UnifiedExec`，实际 shell 能力由 `exec_command` / `write_stdin` 提供。
- 不激活 Codex 原生 task UI；PPD1 保持延期。

## Verification

- `codex-state`: 210/210，doctest 1/1。
- `codex-extension-api`: 3/3、8/8、5/5。
- `codex-taskspace-extension`: 41/41。
- core TaskSpace lib: 75/75；finalization integration: 1/1。
- app-server TaskSpace fork/reload: 1/1；TUI TaskSpace: 3/3。
- core tool-search: 18/18。
- app-server protocol schema fixtures: 6/6，1 ignored。

## Close reason

- 所有确认的工程迁移缺口均已修复并由定向测试验证；剩余 final-wire candidate 的基线晋升属于 W6 发布资格，不属于本故障。
