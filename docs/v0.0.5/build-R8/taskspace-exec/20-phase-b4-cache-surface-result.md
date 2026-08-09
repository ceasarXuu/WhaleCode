# Phase B4 缓存敏感面结果

- Date: 2026-08-09
- Unit: OB-02A
- Status: verified
- Scope: 缓存门禁合同与离线测试；未运行 Whale Agent 或 Provider 请求

## 1. 发现

旧 `tool_declarations` 规则覆盖整个 `core/src/tools/**`。它能发现 Tool schema 变化，但也会把 handler、dispatch、
preflight、settlement 和 tracing 等不会改变 Provider 输入的执行代码判为缓存风险。该范围会制造不必要的门禁命中，
不能准确表达缓存敏感面。

最终 TaskSpace Exec declaration 的生产构建链为：

1. `core/src/tools/spec.rs` 和 `codex-rs/tools/src/**` 定义原生 ToolSpec；
2. `core/src/tools/router.rs` 选择模型可见 Tool，并在 TaskSpace 下替换为 Exec + Hosted Tool；
3. `taskspace_exec/catalog.rs` 从原生 ToolSpec 构造唯一 Exec declaration；
4. `taskspace_exec/map_operations.rs` 定义 Exec 内 Map 操作的 schema；
5. `tool_search_entry.rs` 构造可发现 Tool 的声明信息。

## 2. 门禁边界

`tool_declarations` 已收敛到上述真实声明链。以下执行内部实现不再单独触发缓存门禁：

- `taskspace_exec/dispatch.rs`
- `taskspace_exec/handler.rs`
- `taskspace_exec/preflight.rs`
- `taskspace_exec/response_scope.rs`

如果这些文件同时修改了 `session/turn.rs`、Provider payload、prompt、协议模型或其他既有敏感面，仍会由相应规则命中。
门禁没有增加 TaskSpace 专用 runner，也没有改变 Standard 比较基线。

## 3. 验证

- `python3 -m unittest test_cache_surface_contract.py`：9 passed；
- `python3 -m unittest test_cache_regression_gate.py`：30 passed；
- 正例覆盖原生 spec/router、TaskSpace catalog/map operation 和 Tool protocol；
- 反例覆盖 TaskSpace dispatch/handler/preflight/response scope；
- 未修改 Provider 输入，因此无需真实缓存回归预算。
