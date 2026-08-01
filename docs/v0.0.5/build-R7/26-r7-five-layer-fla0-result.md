# R7 五层架构 FLA-0 基线结果

- 日期：2026-07-20
- 状态：完成
- 行为改动：无
- 冻结生产合同：`48922ce9b`
- 构建来源：`37d5a4e8c`
- 机器结果：[`five-layer-fla0-baseline-result.json`](../../../benchmarks/taskspace/r7/five-layer-fla0-baseline-result.json)

## 1. 本阶段完成内容

1. 从 authority manifest 逐文件重算冻结生产源码与选定 artifact 的 SHA256。
2. 构建并保存现行 `whale` 基线二进制及 attestation。
3. 使用 Docker 对 `single-file-fast-fix` 和 `subscription-billing-repair` 各执行 3 次
   Standard/TaskSpace 配对运行；TaskSpace 固定使用 `map-request`。
4. 保存逐请求 trace、工具事件、Map 事件、验证结果、Token、缓存和时间数据。

## 2. 基线结果

| 样本 | 模式 | 成功 | Request | Input | Uncached input | Cache hit（request 2+） | Wall time |
|---|---|---:|---:|---:|---:|---:|---:|
| simple | Standard | 3/3 | 22 | 255,518 | 16,798 | 未在样本级聚合 | 71,386 ms |
| simple | TaskSpace | 3/3 | 30 | 421,325 | 22,861 | 97.07% | 93,766 ms |
| complex | Standard | 3/3 | 47 | 779,193 | 22,457 | 未在样本级聚合 | 173,161 ms |
| complex | TaskSpace | 3/3 | 52 | 984,340 | 26,516 | 97.24% | 214,385 ms |

6 个 pair 均通过公开与隐藏验证，`engineering_unclean_count=0`。本阶段只建立比较基线，不据此声明
TaskSpace 或五层重构存在性能收益。

## 3. FLA-1 准入

- 冻结源码与选定 artifact 哈希可重算。
- 二进制身份、模型、provider、Docker 和运行目录可追溯。
- 两个开发样本均具备 3 次基线数据。
- FLA-1 可以只改变 ownership 与观测，不改变模型可见 payload。
