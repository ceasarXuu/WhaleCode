# TaskSpace Version Registry

本文档给 TaskSpace E3 评估建立版本号，避免后续讨论时混淆“代码能力版本”“测试运行批次”和“benchmark 结论”。

## 版本规则

- 版本号格式：`MAJOR.MINOR.PATCH`。
- 当前阶段先使用 `0.0.x` 表示实验性 E3 评估版本。
- 每个版本必须绑定：
  - 代码或工程基建状态；
  - 一轮明确的 E3/P0 run root；
  - 样本范围；
  - 诊断性结果；
  - 已缓解问题与新暴露问题。
- 若两个版本之间没有正式代码改动，只是复跑，也必须标记为“复跑版本”，不能把结果差异误解释为工程升级效果。

## 版本列表

| 版本 | 绑定 run | 版本性质 | 范围 | 结论 |
|---|---|---|---|---|
| `0.0.1` | `D:\whalecode-alpha\target\benchp0-20260607-070527` | Remote asset preflight 修复后的 P0 基线版本 | Terminal-Bench P0 candidate 4 samples x 5 repeats；实际 15 pairs，`query-optimize` 0 pairs | 工程路径可跑，fail-closed 生效；诊断性结果 TaskSpace better 3、Standard better 4、both success 4、both failed 4；未达到 clean E3。 |
| `0.0.2` | `D:\whalecode-alpha\target\benchp0-20260607-163444` | 同基建下第二次 P0 复跑版本，用于检查稳定性和复现性 | Terminal-Bench P0 candidate 4 samples x 5 repeats；实际 15 pairs，`query-optimize` 0 pairs | 仍未达到 clean E3；诊断性结果 TaskSpace better 2、Standard better 5、both success 2、both failed 6；新增暴露 validator 容器残留阻塞 driver。 |

## 0.0.1 到 0.0.2 对比

| 维度 | `0.0.1` | `0.0.2` | 判断 |
|---|---|---|---|
| 可执行性 | 3 个可执行样本各完成 5 pair；`query-optimize` 被 fail-closed | 3 个可执行样本各完成 5 pair；`query-optimize` 继续被 fail-closed | 基础执行能力可复现。 |
| Remote asset preflight | 正确阻止 `query-optimize` 的 HuggingFace `oewn.sqlite` | 同样正确阻止 | 已缓解并稳定生效；没有重新 fail-open。 |
| Prompt guard 误杀 | `recover-accuracy-log` 能完整运行，说明早前领域词误杀已缓解 | `recover-accuracy-log` 继续完整运行 | 已缓解并稳定生效。 |
| Final run 收口 | 能完成，但 validator runtime 仍有成本噪声 | 能完成，但中途出现 3 个 validator 容器残留并阻塞 driver，需手动停止本 run 残留容器 | 没有改善；0.0.2 新暴露更明确的 Docker cleanup 缺陷。 |
| Clean E3 aggregate | 0 pair 进入 utility aggregate | 0 pair 进入 utility aggregate | 未改善。human audit / validator gate 仍未形成 clean 路径。 |
| 诊断性方向 | TaskSpace better 3，Standard better 4，both success 4，both failed 4 | TaskSpace better 2，Standard better 5，both success 2，both failed 6 | 0.0.2 结果更差；不能证明 TaskSpace 效用提升。 |
| TaskSpace 图行为 | 已能生成 map/node/edge，部分触发 subagent spawn | 图行为继续发生，且部分任务出现节点/边/subagent result 膨胀 | 机制活跃可复现，但有效性没有随活跃度提升。 |
| 成本噪声 | `apt-get` / validator 安装依赖导致 walltime 噪声 | 同样存在，且部分容器残留导致外层阻塞 | 未改善。 |

## 分 task 对比

| Task | `0.0.1` 结果 | `0.0.2` 结果 | 变化 |
|---|---|---|---|
| `processing-pipeline` | TaskSpace better 0，Standard better 2，both success 1，both failed 2 | TaskSpace better 0，Standard better 3，both success 0，both failed 2 | 变差。TaskSpace 仍无法稳定通过 public validation。 |
| `multi-source-data-merger` | TaskSpace better 2，Standard better 1，both failed 2 | TaskSpace better 0，Standard better 1，both failed 4 | 明显变差。0.0.2 暴露 map/subagent 膨胀与超时更严重。 |
| `recover-accuracy-log` | TaskSpace better 1，Standard better 1，both success 3 | TaskSpace better 2，Standard better 1，both success 2 | 轻微正向但不稳定。它仍是最有潜力的样本。 |
| `query-optimize` | 0 pair，remote asset equivalence unproven | 0 pair，remote asset equivalence unproven | 稳定 fail-closed；未评估 agent 能力。 |

## 工程基建是否生效

| 基建项 | 是否生效 | 证据 | 后续动作 |
|---|---|---|---|
| Remote asset 分类与 fail-closed | 是 | 两轮都阻止 `query-optimize` 的未证明远程 sqlite，同时保留 `uv` cache 证明 | 保持；若要跑 `query-optimize`，需要独立实现资产缓存、hash、注入和等价证明。 |
| Prompt guard 修正 | 是 | `recover-accuracy-log` 在两轮都完整执行 5 pair | 保持；继续防止领域词误杀内部概念过滤。 |
| E3 run index / 结果追踪 | 是 | 两轮均有独立 run root、aggregate、复盘文档 | 继续按版本号记录。 |
| Validator fidelity proof / clean aggregate | 否 | 两轮都是 0 valid utility pairs，仍依赖 manual review / human audit | 需要补 audit review 自动记录与 validator proof 聚合。 |
| Docker validator cleanup | 否 | `0.0.2` aggregate 后仍有容器 running，阻塞 driver | 必须修复：validator 执行应有 finally cleanup、container timeout、孤儿容器回收。 |
| 网络依赖隔离 | 否 | 两轮都受 `apt-get`、`uv`、PyPI 下载影响 | 需要做离线依赖缓存或标记网络噪声。 |
| TaskSpace map 收敛 | 否 | `multi-source-data-merger` 从 0.0.1 的 2 个 TaskSpace better 退化到 0.0.2 的 0 个，且节点/边/subagent result 膨胀 | 需要围绕问题状态管理、节点粒度、停止/收敛策略做机制升级。 |

## 当前结论

`0.0.1` 到 `0.0.2` 说明：底层 E3 执行链路和 remote asset fail-closed 是可复现的；但 TaskSpace 的效用没有提升，反而在 `multi-source-data-merger` 和 `processing-pipeline` 上出现更明显负收益。工程基建里真正已稳定的是资产预检和 prompt guard 修正；尚未解决的是 Docker cleanup、网络依赖噪声、clean E3 gate 和 map 收敛能力。

后续版本应从 `0.0.3` 开始，每次工程机制改动都必须绑定一轮同范围 E3 复跑，并在本文件中记录是否真正改善了上一版本暴露的问题。
