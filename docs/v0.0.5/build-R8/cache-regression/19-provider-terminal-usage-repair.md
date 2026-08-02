# Provider Terminal Usage 事实源修复

- Date: 2026-08-02
- Commit: `0076e720a`
- Status: 离线修复完成；真实双臂复验待新预算
- Related run: `WAR-20260802-165454-CACHE-REGRESSION-2723DE14`

## 1. 问题

缓存 runner 原先同时使用两套计量边界：缓存分项来自 provider wire 派生摘要，token 总量来自 rollout
`token_count` 摘要。Codex 会在工具阶段重复发送相同的累计 token 快照，后者因此把 5 次 provider 请求计算成
9 次，并输出 `108402.0`。严格整数合同正确拒绝该值，但获批运行也因此在 Standard 后停止。

## 2. 修复

现在只有 `provider-wire-trace.jsonl` 的逐请求 terminal usage 是请求数和 token 的事实源：

1. 每个请求必须有唯一 `payload_captured` 和唯一 `response_completed`；
2. usage 字段必须是非负整数，cached input 不得超过 input；
3. request ID、顺序和 payload SHA-256 必须与 provider boundary 完全一致；
4. cache summary 的 request 2+ 分项必须与 terminal usage 复算结果一致；
5. rollout request summary 仍持久化为诊断附件，但不参与费用和缓存结论。

同时，缓存 runner 在 provider-route 之前调用现有 `New-TaskspaceWhaleBinaryHealth`。Python 只负责调用和读取
结果，不复制 attestation 规则；失败时不会进入 provider-route，也不会认领运行授权。

## 3. 验证

| 验证项 | 结果 |
|---|---|
| 原始 Standard artifact 离线重算 | 5 requests；60,617 input；48,128 cached；810 output |
| Request 2+ 缓存命中率 | 97.0812% |
| 估算费用回填 | `$0.0021100184` |
| Python cache-regression suite | 219 passed |
| Ruff | passed |
| 本机 binary-health | passed；attestation passed；0 findings |
| 缓存敏感面 staged gate | passed；指纹 `204978af...` 未变；发布仍阻断 |

本次没有运行新的 Whale Agent。原授权已经消费 1 个 Standard 样本，不能复用；`map-request` 尚无本轮数据。

## 4. 剩余边界

- MVT-0 仍需新预算完成 Standard 与 `map-request` 各一次的同批次比较；
- 通用 rollout request summary 仍会重复计算累计快照，但它已退出缓存计费事实链，可作为独立观测准确性问题后续处理；
- 在真实双臂结果完成前，不晋升 accepted baseline，不解除 release 阻断，也不进入 MVT-1。
