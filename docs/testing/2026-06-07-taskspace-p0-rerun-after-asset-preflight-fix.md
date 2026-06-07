# 2026-06-07 TaskSpace P0 Rerun After Asset Preflight Fix

本文记录修复 Terminal-Bench remote asset preflight 后的一轮 P0/E3 真实样本执行。此轮用于确认工程环境和样本适配性，不作为 clean E3 utility 结论。

- TaskSpace version: `0.0.1`
- Version registry: [taskspace-version-registry.md](./taskspace-version-registry.md)

## 执行范围

- run root: `D:\whalecode-alpha\target\benchp0-20260607-070527`
- source version: `1a6ffa9674b571da0ed040c470cb40c4d85f9b9b`
- model: `deepseek-v4-flash`
- timeout: agent `900s`，validator `420s`
- sandbox: `full-auto`
- repeats: 每个样本 5 pair
- samples:
  - `processing-pipeline`
  - `multi-source-data-merger`
  - `recover-accuracy-log`
  - `query-optimize`

## 总体状态

- driver status: `completed_with_failures`
- 已完成 agent pair: 15/20
- `query-optimize` 未进入 agent 执行，原因是 `oewn.sqlite` 属于未证明等价的远程运行资产，harness 按 fail-closed 处理。
- 0 个 pair 进入最终 utility aggregate。

## 样本结果

| 样本 | 执行结果 | aggregate | 主要结论 |
|---|---:|---|---|
| `processing-pipeline` | 5 pairs | `D:\whalecode-alpha\target\benchp0-20260607-070527\runs\terminal_bench__processing-pipeline\20260607-070532-088\aggregate-report.md` | 3 个 E3-candidate，但都未进入 E3 aggregate；TaskSpace worse 2，both success 1，both failed 2。 |
| `multi-source-data-merger` | 5 pairs | `D:\whalecode-alpha\target\benchp0-20260607-070527\runs\terminal_bench__multi-source-data-merger\20260607-082405-434\aggregate-report.md` | 3 个 E2-candidate、2 个 E1；TaskSpace better 2，worse 1，both failed 2。 |
| `recover-accuracy-log` | 5 pairs | `D:\whalecode-alpha\target\benchp0-20260607-070527\runs\terminal_bench__recover-accuracy-log\20260607-100434-377\aggregate-report.md` | 5 个 E2-candidate；TaskSpace better 1，worse 1，both success 3。 |
| `query-optimize` | 0 pairs | `D:\whalecode-alpha\target\benchp0-20260607-070527\runs\terminal_bench__query-optimize\20260607-112715-551\preflight.remote-assets.json` | `environment_remote_asset_unavailable` / `remote_asset_equivalence_unproven`，未执行 agent。 |

## Pair 结果矩阵

| 样本 | TaskSpace better | Standard better | Both success | Both failed/inconclusive |
|---|---:|---:|---:|---:|
| `processing-pipeline` | 0 | 2 | 1 | 2 |
| `multi-source-data-merger` | 2 | 1 | 0 | 2 |
| `recover-accuracy-log` | 1 | 1 | 3 | 0 |
| 合计 | 3 | 4 | 4 | 4 |

说明：上表是诊断性统计。由于 human audit / validator fidelity gate 未满足，这些结果不能声明为 E3 utility aggregate。

## 工程观察

1. 修复后的 remote asset preflight 生效：`query-optimize` 没有 fail-open，未证明等价的 HuggingFace sqlite 文件被阻止；`uv` validator dependency cache 被正确识别为已证明的非任务资产。
2. `recover-accuracy-log` 此轮完整跑完 5 pair，说明之前 prompt guard 对领域词 `multi-agent` 的误杀已被移除或绕过，不再阻断该样本。
3. Validator runtime 仍有成本噪声：部分样本在容器内安装依赖，尤其 `apt-get` 下载慢，会显著拉长 walltime。
4. TaskSpace 已能生成 map/node/edge，并在部分 pair 中触发 subagent spawn；但本轮还不能证明这些结构稳定带来收益。
5. 当前 E3 gate 仍过严或 proof 记录不完整：pair report 中仍出现 `proof_official_runner_or_equivalent=False`、`proof_validator_e3_eligible=False` 和 `audit_review_missing`，导致所有候选都被排除。

## 后续判断

本轮达成的是“P0 样本能真实跑、fail-closed 生效、部分任务有诊断性正向信号”。未达成 clean E3。下一步应优先补齐：

- validator fidelity proof 的写回与聚合口径；
- audit-review 的自动化记录路径；
- 对 validator runtime 成本噪声的隔离或标注；
- 对 TaskSpace map 生长质量的人工复核模板。
