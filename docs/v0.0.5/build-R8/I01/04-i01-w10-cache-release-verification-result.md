# R8 I01-W10 缓存发布验收结果

- Date: 2026-08-18
- Subject commit: `413f940d7e846ffd029619fe38ba014fcf38736b`
- Result: `WAR-20260818-222826-CACHE-REGRESSION-D4B2E4BE`
- Status: accepted / baseline promoted
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`

## 1. 验收范围

本轮只验证 I01 final-wire 变化实际影响的三个 TaskSpace projection。Standard 的 final-wire 在免费门禁中没有变化，
因此没有重复购买 Standard 运行。三个模式各运行一次，零重试；任一业务、usage、预算或运行异常均应立即停止。

## 2. 结果

| 模式 | 业务 | 请求 | Input | Cached | Uncached | Output | Request 2+ 命中率 |
|---|---:|---:|---:|---:|---:|---:|---:|
| map-always | PASS | 7 | 107,269 | 89,472 | 17,797 | 2,407 | 87.53% |
| map-append | PASS | 7 | 121,365 | 114,688 | 6,677 | 2,728 | 93.97% |
| map-request | PASS | 8 | 123,866 | 114,688 | 9,178 | 2,494 | 91.84% |
| **合计** | **3/3** | **22** | **352,500** | **318,848** | **33,652** | **7,629** | - |

完整 usage 覆盖率为 100%，无未验证范围，无停止条件命中。总耗时 178.225 秒，按冻结价格估算费用为
`CNY 0.05528696`。

## 3. 结论边界

本轮证明当前提交上的三种 projection 均能完成业务任务，且当前 final-wire 候选具有完整、可复算的 Provider 缓存证据。
它不证明三种模式具有相同成本，也不替代 I08 的产品阈值决策。

用户已明确接受该精确结果，缓存发布脚本已在 subject commit 仍为当前 HEAD 时完成晋升。接受范围仅限本文件列出的提交、
模型、样本、三个模式与单次运行；不代表选择默认 projection，也不关闭 I03、I04、I07 或 I08。

## 4. 证据

- Result: `benchmarks/cache-regression/results/WAR-20260818-222826-CACHE-REGRESSION-D4B2E4BE.json`
- Proposal: `benchmarks/cache-regression/proposals/2026-08-18-r8-i01-w10-current-head.json`
- Authorization: `benchmarks/cache-regression/authorizations/CBA-20260818-R8-I01-W10-9D74CF85E2C39355.json`
- Acceptance: `benchmarks/cache-regression/acceptances/WAR-20260818-222826-CACHE-REGRESSION-D4B2E4BE.json`
- Evidence root: `benchmarks/cache-regression/evidence/WAR-20260818-222826-CACHE-REGRESSION-D4B2E4BE/`
- Global ledger: `benchmarks/whale-agent-run-ledger.json`

启动前发生三次 Provider 零请求的本地预检失败：提案 HEAD 身份不一致、授权合同版本/摘要不一致，以及相对
`--run-root` 触发 runner 路径处理错误。它们没有消耗模型预算，不计入本轮实际运行；正式运行改用绝对 run root 后通过。

## 5. I01 关闭结论

I01 的旧双 revision 根因已移除，三种 projection 共享唯一 final result；确定性合同、复杂样本 9/9、W10 缓存发布运行与
accepted baseline 均已通过。I01-W11 完成，I01 状态更新为 `closed`。
