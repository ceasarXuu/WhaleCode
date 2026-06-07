# 2026-06-07 TaskSpace P0 Second Rerun

本文记录同日第二轮 P0/E3 真实样本执行。此轮继续使用 Terminal-Bench P0 candidate 范围，用于观察稳定性、可重复性和 harness 剩余噪声。

- TaskSpace version: `0.0.2`
- Version registry: [taskspace-version-registry.md](./taskspace-version-registry.md)

## 执行范围

- run root: `D:\whalecode-alpha\target\benchp0-20260607-163444`
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
- `query-optimize` 未进入 agent 执行，原因仍是 HuggingFace `oewn.sqlite` 未证明等价，remote asset preflight 按 fail-closed 阻止。
- 0 个 pair 进入最终 utility aggregate。
- 本轮中途发生一次 harness 收口问题：`multi-source-data-merger` 已写出 5/5 pair-report 和 aggregate 后，3 个 validator Docker 容器仍保持 running，阻塞 driver 进入下一个样本。已记录后停止这 3 个本 run 残留容器，driver 随即继续执行。

## 样本结果

| 样本 | 执行结果 | aggregate | 主要结论 |
|---|---:|---|---|
| `processing-pipeline` | 5 pairs | `D:\whalecode-alpha\target\benchp0-20260607-163444\runs\terminal_bench__processing-pipeline\20260607-163451-317\aggregate-report.md` | 3 个 E3-candidate、2 个 E1；0 个进入 aggregate。方向上 Standard better 3、both failed 2。 |
| `multi-source-data-merger` | 5 pairs | `D:\whalecode-alpha\target\benchp0-20260607-163444\runs\terminal_bench__multi-source-data-merger\20260607-181021-823\aggregate-report.md` | 1 个 E2-candidate、4 个 E1；0 个进入 aggregate。方向上 Standard better 1、both failed 4。 |
| `recover-accuracy-log` | 5 pairs | `D:\whalecode-alpha\target\benchp0-20260607-163444\runs\terminal_bench__recover-accuracy-log\20260607-202431-017\aggregate-report.md` | 5 个 E2-candidate；0 个进入 aggregate。方向上 TaskSpace better 2、Standard better 1、both success 2。 |
| `query-optimize` | 0 pairs | `D:\whalecode-alpha\target\benchp0-20260607-163444\runs\terminal_bench__query-optimize\20260607-215541-538\preflight.remote-assets.json` | `environment_remote_asset_unavailable` / `remote_asset_equivalence_unproven`。 |

## Pair 结果矩阵

| 样本 | TaskSpace better | Standard better | Both success | Both failed/inconclusive |
|---|---:|---:|---:|---:|
| `processing-pipeline` | 0 | 3 | 0 | 2 |
| `multi-source-data-merger` | 0 | 1 | 0 | 4 |
| `recover-accuracy-log` | 2 | 1 | 2 | 0 |
| 合计 | 2 | 5 | 2 | 6 |

说明：上表是诊断性统计。由于 `manual_review_required`、`e3_human_review_not_completed`、`docker_run_failure` 或 `business_success_false` 等 gate 未满足，不能声明为 clean E3 utility 证据。

## TaskSpace 行为观察

- `processing-pipeline` 中 TaskSpace 多次生成 map/node/edge，并触发 subagent spawn，但 public validation 失败，方向上弱于 Standard。
- `multi-source-data-merger` 出现明显超时与成本放大：部分 TaskSpace pair 达到 900s，节点数可到 21/22，边数可到 33/36，subagent result 可到 50，但没有换来业务成功。
- `recover-accuracy-log` 是本轮唯一出现 TaskSpace 正向信号的样本：5 pair 中 TaskSpace better 2、both success 2、Standard better 1。
- `query-optimize` 的 fail-closed 行为稳定，说明 remote asset preflight 没有重新放开未证明远程运行资产。

## 工程问题

1. Validator Docker cleanup 仍不可靠：aggregate 已完成后仍可能留下运行容器，并阻塞外层 driver。
2. Public validator 依赖网络安装包，容易引入 `apt-get`、`uv`、PyPI 下载超时；这会污染 walltime 和 business_success。
3. E3 gate 仍未形成 clean aggregate 路径：缺 human audit 自动记录，部分 pair 仍因 validator runtime/env 问题被排除。
4. TaskSpace 在数据合并类任务上有过度分解和长时间运行迹象，需要继续检查 map 生长策略。

## 结论

本轮证明的是执行路径可重复跑通，并进一步确认 P0 样本下 TaskSpace 仍未达到 clean E3。相比上一轮，本轮 `recover-accuracy-log` 仍保留正向信号，但 `processing-pipeline` 和 `multi-source-data-merger` 继续暴露负收益和 harness 噪声。
