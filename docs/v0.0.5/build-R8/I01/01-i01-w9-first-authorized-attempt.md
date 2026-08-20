# R8-I01 W9 首次授权运行记录

- 时间：2026-08-01 21:55（Asia/Shanghai）
- 账本记录：`WAR-20260801-215033-R8-I01-W9-9B49F6DC`
- 产品提交：`9b49f6dc96ad553ab454fefc2c96c975a6838442`
- 执行 HEAD：`5ea7234e9b6860d467fc9decd4a0e8babc2cdc29`
- 状态：preflight failed；0 sample、0 provider request、0 token、0 费用

## 1. 获批范围

- 模型：`deepseek-v4-flash`
- Sample：`single-file-fast-fix`
- Arms：`map-always`、`map-append`、`map-request`
- Repeat：每臂 3 次，共 9 runs
- 每 run provider request 硬上限：10；总硬上限：90
- 自动重试：0
- 停止条件：任一 run/业务失败、usage 缺失或观测阈值越界

## 2. 实际发生路径

1. 全局账本先写入 `planned`，合同测试通过并提交。
2. Whale binary 从干净 HEAD 重建；attestation 校验通过，binary SHA-256 为
   `45d37c43cef498e2ed075856ee6f2d631f85c35efedf7f05d04fec9165625aa0`。
3. 第一个 `map-always / repeat 1` 在 workspace materialization 后、pair dispatch 前执行 treatment-neutral cwd
   检查。
4. RunRoot 使用了仓库内路径 `target/r8-i01-w9/.../map-always/...`，生成的左右 repo 路径包含 `/map`，被
   `Test-TaskspaceNeutralCwd` 拒绝。
5. 批次按 `after_any_run_failure` 立即停止；其余 8 个计划 run 未启动，也没有自动重试。

## 3. 成本与证据

- `sample-status.json`：`attempted_pairs=0`、`completed_pairs=0`。
- 运行目录不存在 provider-boundary events/evidence 和 provider wire trace。
- Docker 中不存在该 RunId 的容器或网络残留。
- 因失败发生在 provider boundary 创建前，账本精确结算 `api_requests=0`，token 与估算费用均为 0。
- 原始证据目录：
  `target/r8-i01-w9/WAR-20260801-215033-R8-I01-W9-9B49F6DC/map-always/r-1/`。

## 4. 根因与经验复用

这是执行编排错误，不是 I01 产品修复、Agent、模型或 provider 的结果。中性 cwd 门禁有意禁止路径暴露
`standard`、`taskspace`、`map`、`node` 等 treatment 标签，避免模型从工作目录推断实验分组。

该约束已在历史 `build-R7/08-r7-phase-d-result.md` 中记录，但本次执行前没有复用，说明运行前检查清单缺少对已知
经验的机械核对。下一次执行必须满足：

1. Execution/RunRoot 使用仓库外的中性路径，例如 `/tmp/wrun/<batch>/a1/r1`；路径本身不含 treatment 标签。
2. 在任何 provider run 前，先用同一 runner 的 `PlanOnly` 或纯 workspace fixture 验证左右 repo cwd 均通过
   `Test-TaskspaceNeutralCwd`。
3. 继续使用三种 policy 参数传递实验分组，不把 policy 名写入 Agent 可见路径。
4. 首次授权已经按零重试合同结算；修正路径后的真实运行必须建立新账本记录并重新获得用户批准。

## 5. I01 结论边界

本次没有产生真实 Agent 行为证据，不能支持或反驳 W9 的 `hidden_attribution_stale=0` 目标。I01 保持
`verifying`，W9 仍未完成，W10 也未开始。
