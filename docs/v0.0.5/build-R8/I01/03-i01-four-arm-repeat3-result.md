# R8 四臂 Projection Repeat-3 结果

- Date: 2026-08-18
- Run record: `WAR-20260818-055427-R8-FOUR-ARM-R3`
- Commit under test: `4fe2f3557eab1ca07836dfdc9e0f909b73329ea7`
- Model: `deepseek-v4-flash`
- Sample: `subscription-billing-repair`
- Arms: Standard、`map-always`、`map-append`、`map-request`
- Repeats: 每臂 3 次，共 12 次正式运行
- Durable evidence: `benchmarks/taskspace/r8/evidence/WAR-20260818-055427-R8-FOUR-ARM-R3.json`

## 1. 结论

四臂均为 **3/3 业务成功**，公开验证和隐藏 oracle 均通过，12 次产生完全相同的 4 个变更路径。三种
TaskSpace 模式的 9 张 Map 全部闭合、无 open leaf、无图健康警告，也没有出现 `stale_revision`。这支持
I01-W9 的核心结论：三种 projection policy 共用的最终进度事实没有因 policy 切换形成分叉。

性能上不存在单一全面最优模式：

- `map-always` 的总 input 仅为 Standard 的 `1.25x`，但动态替换 projection 使 request 2+ 缓存命中只有
  `84.21%`，实际估算费用是三个 TaskSpace 模式中最高的；
- `map-append` 的总 input 最大，为 Standard 的 `1.60x`，但缓存命中达到 `93.49%`，因此实际费用反而是
  三个 TaskSpace 模式中最低的；
- `map-request` 没有自动 projection，input 为 Standard 的 `1.32x`，但本轮自然执行历史较长，缓存命中
  `89.39%`，费用没有低于 `map-append`。

本轮同时复现 3 次零副作用、下一请求可恢复的 TaskSpace 协议拒绝。三种模式可以完成复杂 client-tool 任务，
但 I03/I04 还不能关闭。报告工具还暴露两个 I07 观测缺口，不能继续把 I07 标为完全关闭。

## 2. 汇总对比

| 模式 | Success | Requests | Input | 平均单请求 Input | Cached | Uncached | Output | Request 2+ cache | 最终请求 cache 均值 | Agent wall | 估算费用 CNY |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Standard | 3/3 | 30 | 442,990 | 14,766 | 433,664 | 9,326 | 13,981 | 97.80% | 99.31% | 116.527s | 0.04596128 |
| map-always | 3/3 | 30 | 553,757 | 18,459 | 472,064 | 81,693 | 19,542 | 84.21% | 91.18% | 162.324s | 0.13021828 |
| map-append | 3/3 | 32 | 710,150 | 22,192 | 666,112 | 44,038 | 19,421 | 93.49% | 95.65% | 155.406s | 0.09620224 |
| map-request | 3/3 | 31 | 586,416 | 18,917 | 528,000 | 58,416 | 21,268 | 89.39% | 94.75% | 171.306s | 0.11151200 |

相对 Standard：

| 模式 | Request | 总 Input | 平均单请求 Input | Uncached | Agent wall | 费用 |
|---|---:|---:|---:|---:|---:|---:|
| map-always | 1.00x | 1.25x | 1.25x | 8.76x | 1.39x | 2.83x |
| map-append | 1.07x | 1.60x | 1.50x | 4.72x | 1.33x | 2.09x |
| map-request | 1.03x | 1.32x | 1.28x | 6.26x | 1.47x | 2.43x |

12 次合计 123 个 Provider 请求、2,293,313 input、74,212 output，按冻结价格估算 CNY `0.3838938`，
低于获批的 CNY `0.96` 硬上限。运行无自动重试。

## 3. 单轮明细

| 模式 | Repeat | Requests | Input | Cached | Uncached | Output | Request 2+ cache | 最终请求 cache | Wall | Map nodes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Standard | 1 | 9 | 130,028 | 127,104 | 2,924 | 4,342 | 97.63% | 99.12% | 35.970s | N/A |
| Standard | 2 | 12 | 182,440 | 178,944 | 3,496 | 5,651 | 98.02% | 98.91% | 46.347s | N/A |
| Standard | 3 | 9 | 130,522 | 127,616 | 2,906 | 3,988 | 97.65% | 99.91% | 34.210s | N/A |
| map-always | 1 | 10 | 177,410 | 152,192 | 25,218 | 6,411 | 84.79% | 92.46% | 47.558s | 5 |
| map-always | 2 | 11 | 209,710 | 179,584 | 30,126 | 7,248 | 84.73% | 92.20% | 64.211s | 6 |
| map-always | 3 | 9 | 166,637 | 140,288 | 26,349 | 5,883 | 82.91% | 88.89% | 50.555s | 6 |
| map-append | 1 | 10 | 212,375 | 198,016 | 14,359 | 5,643 | 92.90% | 95.68% | 42.967s | 5 |
| map-append | 2 | 12 | 288,169 | 272,000 | 16,169 | 7,696 | 94.15% | 95.42% | 62.550s | 5 |
| map-append | 3 | 10 | 209,606 | 196,096 | 13,510 | 6,082 | 93.17% | 95.85% | 49.889s | 6 |
| map-request | 1 | 10 | 181,163 | 163,712 | 17,451 | 6,346 | 89.74% | 98.21% | 52.909s | 5 |
| map-request | 2 | 11 | 211,214 | 191,360 | 19,854 | 8,163 | 90.02% | 93.87% | 61.514s | 6 |
| map-request | 3 | 10 | 194,039 | 172,928 | 21,111 | 6,759 | 88.38% | 92.18% | 56.883s | 6 |

## 4. Projection 生效证据

以 provider final-wire section scanner 为权威：

| 模式 | Requests | Bootstrap projection | Active projection | Projection unavailable |
|---|---:|---:|---:|---:|
| map-always | 30 | 4 | 26 | 0 |
| map-append | 32 | 3 | 29 | 0 |
| map-request | 31 | 0 | 0 | 31 |

这证明三臂不是只改了配置标签。`map-always` 与 `map-append` 的 projection 确实进入最终 Provider 请求，
`map-request` 则按设计不自动注入。

`map-append` 的 projection section 累计估算为 125,268 tokens，显著高于 `map-always` 的 22,842 tokens，
对应“旧 projection 持续留在自然追加历史”的已知产品代价。它同时获得更高缓存命中，说明 input 浪费与缓存收益
必须同时计价，不能只看 token 总量或缓存百分比中的一个。

## 5. Map 与动作

- 三种 TaskSpace 模式每轮创建 5～6 个节点、4～5 条边，Root 与 Finish 唯一；9/9 Root completed，open leaf 为 0。
- Map 基本为 `root -> explore/understand -> diagnose/fix -> verify -> finish` 的线性依赖链。样本虽然修改 4 个文件，
  但修复共享同一 README 规则和一次集成验证，Agent 合并为一个 fix 节点是合理的。
- 本轮没有覆盖 fork/join、多 Ready 节点或多父节点，因此不能据此关闭 I04 的复杂 frontier 验收。
- 12 次均只产生一次补丁请求，最终变更路径一致；没有 request-wide multi-patch。

## 6. 异常

### 6.1 TaskSpace 协议拒绝

| 模式 | Repeat | 表现 | Runtime 行为 | 后续 |
|---|---:|---|---|---|
| map-always | 1 | `initialize_map` 类型不符合合同 | 零 Map/Tool 副作用拒绝 | 下一请求纠正并完成 |
| map-request | 2 | 在父节点未完成时执行 waiting 节点 | 零 Map/Tool 副作用拒绝 | 下一请求纠正并继续 |
| map-request | 2 | Patch 字符串含未转义换行，严格 JSON 解析失败 | 零 Map/Tool 副作用拒绝 | 下一请求纠正并完成 |

发生频率为 2/9 TaskSpace runs、3 次拒绝。硬边界和反馈恢复有效，但这不是“零异常”结果：I03 继续 verifying，
I04 明确复现 1 次 waiting/frontier 误选。JSON 自愈器没有覆盖这次多行 Patch 内部控制字符，后续应单独判断是否属于
可机械、唯一修复的输入，而不是放宽 JSON 合同。

### 6.2 普通命令错误

Standard 2 次、`map-append` 1 次直接执行 `python` 时未使用项目 `pythonpath=src`，产生
`ModuleNotFoundError: billing_service`。这些错误没有改变最终业务结果，属于 Agent 命令选择问题，不是 TaskSpace
状态机或 projection 特有缺陷。

## 7. Observer 缺口

本轮发现两个已有观测路径未跟上当前 final wire 的问题：

1. `provider-cache-trace-summary.json` 正确观测 `map-always/map-append` projection；旧
   `context-projection-summary.json` 和顶层 metrics 仍从 `whale-exec` 推导并报告 `projection_unavailable`。
2. 三次明确的 `taskspace_exec rejected` 被计入 `failed_tools`，但 `sequence_preflight_rejected_calls`、
   `control_*_failures`、`nested_action_failures` 仍全部为 0；`exec_findings` 也只识别了 JSON 一次。

因此成本、usage、Map 和 provider final-wire 结果仍可用，但 projection 与拒绝子类型必须读取权威 trace，不能依赖旧
派生字段。I07 从 closed 回到 verifying，修复只应统一 observer 事实源和分类，不改 Runtime 产品行为。

后续状态：该缺口已按上述边界修复，并使用本轮原始 trace 离线回放通过；见
[`I07 final-wire observer 增量修复结果`](../I07/02-i07-final-wire-observer-repair-result.md)。历史运行生成时的旧报告保留，
不反向改写原始证据。

## 8. 问题重评

| 问题 | 本轮结论 |
|---|---|
| I01 | W9 completed：三 policy 9/9 成功、stale revision 为 0；W10 发布缓存证据仍按原计划独立结算，暂不直接关闭 |
| I03 | 复杂 client-tool 样本 9/9 完成，但 2/9 出现可恢复协议错误，保持 verifying |
| I04 | 复现 1 次 waiting/frontier 误选；硬门正确，Agent 状态掌握仍不稳定，保持 verifying |
| I07 | 新发现 projection 与 reject 分类漏报，重新 verifying |
| I08 | 复杂样本四臂成本已测；三种模式固有取舍得到量化，产品阈值和更多样本外推仍未完成 |

## 9. 执行准备说明

正式运行前有两组本地预检失败：第一组为二进制 attestation 哈希不一致，第二组为运行路径包含 treatment 名称而被
cwd 防泄漏门禁拒绝。两组均未发出 Provider 请求，不计入 12 次正式 repeat，也没有产生 API 费用。正式矩阵改用既有
匿名臂编码 `a0/a1/a2/a3` 后执行。
