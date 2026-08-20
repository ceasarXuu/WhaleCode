# MVT-0 双臂基线接受结果

- Date: 2026-08-02
- Run: `WAR-20260802-181842-CACHE-REGRESSION-7A794B3A`
- Subject commit: `0c9dd07aa`
- Status: accepted

## 1. 接受范围

用户接受本轮 `single-file-fast-fix` 的 Standard 与 map-request 各一次结果，作为 Tool Sequence MVT-0 当前请求与缓存
基线。接受范围仅包括：

- `deepseek-v4-flash`；
- `single-file-fast-fix`；
- Standard、map-request 各 `repeat=1`；
- 三种 TaskSpace policy 当前两请求 final-wire 快照；
- 本轮实际 provider usage、业务结果和运行身份。

该接受不表示 map-request 成本已经达标，不关闭 R8-I02、R8-I05 或 R8-I08，也不把一次样本提升为稳定性能结论。

## 2. 双臂结果

| 指标 | Standard | Map Request | Map Request / Standard |
|---|---:|---:|---:|
| 业务结果 | 通过 | 通过 | - |
| Provider 请求 | 7 | 8 | 1.14x |
| Agent wall time | 14.58s | 32.14s | 2.20x |
| Input token | 85,437 | 133,706 | 1.57x |
| Cached input | 83,840 | 87,552 | 1.04x |
| Uncached input | 1,597 | 46,154 | 28.90x |
| Output token | 1,171 | 2,814 | 2.40x |
| Request 2+ cache hit | 97.90% | 67.85% | -30.05pp |
| 估算费用 | `$0.000786212` | `$0.0074946256` | 9.53x |

批次合计 15 次 provider 请求、219,143 input、171,392 cached input、47,751 uncached input、3,985 output，
估算费用 `$0.0082808376`，实际运行结算 114.633 秒。两臂 usage 覆盖率均为 100%，业务和清理均通过。

## 3. Trace 事实

map-request 创建 1 张 Map、5 个节点和 4 条边，根任务与 finish 均闭合，无 open leaf。Agent 第一次尝试 Patch 时，
`fix` 仍在等待前置 `explore`；Runtime 在零执行状态下返回 `node_state_invalid`。Agent 下一请求完成 `explore`，再把同一
Patch 绑定到 `fix` 并成功执行，随后测试通过并手动 `finish_map`。

因此多出的 1 次请求来自一次可纠正的前置依赖误判，不是 Patch handler 执行失败。相同拒绝事实同时进入 control
output、Patch output 和 developer message，继续作为 R8-I05/I02 的已知反馈重复问题，不在 MVT-0 中修复。

## 4. Promotion

正式 promotion：

- 把 `cache-surface-contract.json` 基线状态更新为 `accepted`；
- 只替换 map-always、map-append、map-request 三份受保护 TaskSpace final-wire 快照；
- Standard 快照保持不变；
- acceptance 明确绑定 result、proposal、authorization、gate report、ledger 和原始 provider terminal evidence；
- 后续缓存敏感变更继续相对本次 accepted surface 执行门禁。

MVT-0 至此完成。下一项是 MVT-1：仅用本地 Rust 测试证明序列容器中的普通 Tool 能复用现有 `ToolRouter`，不启动
真实 Whale Agent，不修改普通 Tool schema，也不建设第二套 handler。
