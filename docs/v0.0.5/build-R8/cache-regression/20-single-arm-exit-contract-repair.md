# 单臂缓存回归退出合同修复

- Date: 2026-08-02
- Commit: `c2246a6f1`
- Status: 离线修复完成；真实双臂复验待新预算
- Related run: `WAR-20260802-180016-CACHE-REGRESSION-2E8B3F50`

## 1. 本轮事实

用户批准 `single-file-fast-fix` 的 Standard 与 map-request 各一次，禁止重试。专用 runner 完成 Standard 后按停止
条件中止，没有启动 map-request：

| 项目 | Standard 实际值 |
|---|---:|
| 业务结果 | 通过 |
| Provider 请求 | 6 |
| Input token | 74,555 |
| Cached input token | 72,960 |
| Uncached input token | 1,595 |
| Output token | 1,293 |
| Request 2+ 命中率 | 97.5422% |
| 耗时 | 33.428 秒 |
| 估算费用 | `$0.000789628` |

usage 覆盖完整、provider boundary 对账通过、预算未超限、容器/网络/secret 清理均验证为空。`sample-status.json`
记录 `phase=completed`、`exit_code=0`，选中侧 `metrics.json` 记录 `business_success=true`。

## 2. 根因

缓存 runner 使用 `-RunSide left/right` 每次只观测一个 arm，但底层通用 benchmark 默认按完整双臂 E2 证据决定最终
进程退出码。Standard 单臂完成后，右臂按计划被跳过；双臂报告因此不满足 E2，并返回退出码 1。缓存 runner 又把任何
非零退出码机械判为运行失败，于是正确触发停止条件。

这是测试合同冲突，不是 Agent、provider、缓存或业务执行失败：

- 单臂 cache smoke 的目标是取得选中 arm 的完整 usage 与业务结果；
- 双臂 E2 报告的目标是形成可评分的成对证据；
- 单臂运行按定义无法形成双臂 E2，不能把这个预期缺失伪装成选中 arm 失败。

## 3. 修复边界

缓存专用命令现在显式传入已有的 `-AllowNonE2Result`。没有修改通用 benchmark、TaskSpace 产品语义、Tool、provider
请求或上下文。该参数只跳过脚本末尾的 E2 目标不满足退出；以下现有失败仍然保持阻断：

- binary、provider credential、route 和 harness 预检失败；
- 子进程异常、超时或清理不完整；
- 选中 arm `business_success=false`；
- provider terminal usage 缺失或 boundary 对账失败；
- 请求、token、耗时观测超过授权边界。

因此修复不是忽略 benchmark 失败，而是让双臂评分资格退出 cache 单臂执行合同；选中 arm 的执行和证据资格继续由
缓存 runner 的专用硬门禁判定。

## 4. 验证与剩余边界

| 验证项 | 结果 |
|---|---|
| Python cache-regression suite | 219 passed |
| Ruff | passed |
| `git diff --check` | passed |
| 缓存敏感面 staged gate | passed；指纹仍为 `204978af...`；政策变化待真实验证，发布保持阻断 |

本轮授权已经消费，不能在修复后复用。map-request 仍未验证，MVT-0 保持 blocked；后续需要新预算重新运行同批次
Standard + map-request，才能判断 MVT-0 是否完成并处理 accepted baseline。
