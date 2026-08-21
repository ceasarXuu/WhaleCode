# U8：恢复 provider 用量对账与开发期请求硬门禁

- 日期：2026-08-14
- 上游 substrate：Codex CLI `rust-v0.147.0`
- 结论：`verified`
- 真实模型请求：0

## 1. 实施边界

本单元恢复两个彼此相邻的 provider accounting 合同：

1. DeepSeek `response.completed` usage 必须原样通过 core stream，并在 terminal 时写入 rollout inference 记录；
2. 只有显式设置 `WHALE_PROVIDER_REQUEST_HARD_LIMIT` 的获批开发回归，才启用 transport-exact 请求硬门禁。

请求门禁是开发流程的基础保护，不是 Whale 产品授权逻辑：

- 普通运行不设置环境变量，行为完全不变；
- 不解析自然语言，不读取批准文本，也不替代全局 run ledger；
- 限额由外部 runner 注入，provider client 只执行机械计数与 fail-closed；
- 真实运行仍必须遵守 `AGENTS.md` 的预算批准和账本规则。

本单元不修改 compaction 阈值、TaskSpace request budget、缓存策略或模型目录。

## 2. 最小实现

在 0.147 的既有 `ModelClientState` 和 transport seam 上恢复局部 guard：

- `WHALE_PROVIDER_REQUEST_HARD_LIMIT` 必须为正整数；非法配置在 dispatch 前失败；
- 可选 `WHALE_PROVIDER_REQUEST_HARD_LIMIT_STATE_PATH` 通过 Unix 文件锁让同一获批 run 内的多个 client/agent 共享计数；
- 启用硬门禁时把 provider request/stream retry 设为 0，避免隐藏重试绕过真实请求数；
- HTTP Responses、Responses WebSocket、compact、memory summarize 和 Realtime call 在真实 dispatch 前 claim；
- Realtime session 可能由服务端自动生成响应，无法 transport-exact 计数，因此硬门禁启用时统一拒绝；
- 未启用时不改变任何普通 Realtime 行为。

用量链继续复用 0.147 上游 `ResponseEvent::Completed -> map_response_events -> InferenceTraceAttempt::record_completed`，只增加回归 fixture，不增加第二套 accounting 状态。

## 3. 验证结果

| 验证 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | passed；仅 stable rustfmt 的已知 unstable-option warning |
| provider hard-limit 定向测试 | 5 passed：精确上限、非法配置、禁用隐藏重试、跨 client 共享、Realtime fail-closed |
| core client tests | 20 passed |
| core Realtime tests | 15 passed |
| provider usage terminal reconciliation | passed；stream event 与 rollout terminal usage 一致 |
| sync replay / metadata 门禁 | 42 tests passed；inventory/replay/metadata checks passed；当前 overlay 21 路径 |
| cache regression index gate | passed；指纹 `356defc3cc7333e0a3b3de8a8ad8d3ae8b545483422272edf175e67ab8dbdd4f`；最近一次 live 回归仍为失败 |
| 真实网络/API 请求 | 0 |

## 4. 结论

U8 已把旧实现中仍有效的开发期成本保护迁移到 0.147 seam，同时明确排除 TaskSpace budget 状态和产品授权语义。下一工作单元为 U9：恢复 DeepSeek compaction 与长上下文阈值合同。
