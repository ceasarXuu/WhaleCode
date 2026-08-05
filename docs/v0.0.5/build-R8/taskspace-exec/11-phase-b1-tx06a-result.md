# Phase B1 TX-06A 执行结果

- Date: 2026-08-06
- Unit: TX-06A 唯一中性 ToolSpec capability projection
- Status: Complete
- Paid Whale Agent run: 未执行

## 1. 结论

原 `collect_code_mode_exec_prompt_tool_definitions` 不能原样作为共享中立层。它同时承担了两类 Code Mode
专属策略：排除 `exec/wait`，以及将 Freeform grammar 改写为 Code Mode 描述。TaskSpace 直接依赖它会把另一模式的
容器策略误当作 Tool 事实。

本单元新增 `ToolSpecCapability` 与 `project_tool_spec_capabilities`，只机械投影：

1. Function 的原名称、参数 schema、输出 schema 和 deferred 属性；
2. Freeform 的原描述和 grammar contract；
3. Namespace child 的原生 `ToolName` 与确定性公开名称；
4. Provider-hosted 等非本单元目标类型不进入 schema-backed client capability 集合。

Code Mode 与 TaskSpace catalog 现在消费同一中立投影。`exec/wait` 与 `taskspace_exec` 的递归或无效调用排除由各自
容器显式负责，不再隐式借用另一模式的规则。普通 ToolSpec、handler、Router 和 Standard provider wire 均未修改。

## 2. 工程影响

| Surface | Before | After |
|---|---|---|
| ToolSpec 转换事实 | 私有地混在 Code Mode 策略中 | 一份中立 capability projection |
| Code Mode | 直接转换并过滤 | 中立投影后执行原有过滤和描述适配 |
| TaskSpace catalog | 调用 Code Mode 专属 collector | 调用中立投影并显式执行 TaskSpace 容器规则 |
| Tool schema / handler | 无变化 | 无变化 |
| Agent 可见 TaskSpace schema | source-only 隔离候选 | 本单元不变，留给 TX-06B |

## 3. 验证证据

| Check | Result |
|---|---|
| `cargo test -p codex-tools --quiet` | 154 passed / 1 ignored |
| `cargo test -p codex-core taskspace_exec --quiet` | 33 passed |
| `cargo fmt --all -- --check` | PASS |
| `python3 scripts/cache-regression/check_cache_regression_gate.py --source index` | PASS |
| Cache surface SHA-256 | `de2c8b035166e3203bf6bd7df7ecf340dd6c5c3bfa6c5543bb001ebf530e3e7a` |

新增测试明确锁定：中立层不排除名为 `exec` 的原始 capability；Code Mode 自己排除 `exec/wait`；TaskSpace
自己排除 `taskspace_exec/exec/wait`；Function、Freeform、Namespace 和 hosted 排除均有独立 fixture。

## 4. 未被本单元证明的内容

1. 最终结构化 `taskspace_exec` schema 尚未生成；
2. TaskSpace 顶层单次 Tool 合同暴露尚未接生产请求；
3. DeepSeek 对最终 schema 的生成兼容性尚未验证；
4. response envelope、Map admission、Router dispatch 和 Hosted persistence 均未接入。

下一单元为 TX-06B，仅建设隔离的最终结构化 outer schema 与确定性 fixture，不提前切换生产 projection。
