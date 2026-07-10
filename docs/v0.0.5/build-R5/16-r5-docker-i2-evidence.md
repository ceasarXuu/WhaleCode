# R5-I2 Docker 全链迁移验收

## 1. 结论

R5-I2 已通过。Standard、TaskSpace Agent、public validator 和 hidden oracle 已进入同一 Docker
编排路径；宿主机只负责容器编排、临时 secret、artifact 保存和机械汇总。该阶段不声明 TaskSpace
cadence 收益，真实请求放大基线转交 R5-J。

## 2. 实现范围

| 模块 | 结果 |
|---|---|
| Agent | Standard/R5 使用同一 image、资源、路径、provider config，仅 `--taskspace` 不同 |
| Validator | 独立容器执行，写入 tests started/completed 生命周期标记 |
| Oracle | 独立只读 workspace 与私有 `/oracle/oracle.py` mount |
| Secret | `.env.local` 仅在宿主临时文件中落地，权限600，只读挂载，运行后删除 |
| Logs | container manifest、lifecycle、inspect、stdout/stderr、stats、cleanup 全部落入 side artifacts |
| Rollout | 容器 session rollout 复制到 `/artifacts/rollout.jsonl`，exporter 优先读取该路径 |
| 用户权限 | 容器使用宿主 UID/GID，artifact 不再产生 root-owned 不可读目录 |
| 环境隔离 | 固定关闭 benchmark 无关的 plugins、bundled skills 与 skills instructions |

## 3. 真实 paired smoke

证据目录：
`target/r5-i2-docker-paired-clean/count-call-stack/20260711-035022-462`。

| 指标 | Standard | R5 |
|---|---:|---:|
| 业务结果 | solved | solved |
| Agent completion | complete | complete |
| Public validation | 0 | 0 |
| Hidden oracle | 0 | 0 |
| Requests | 7 | 16 |
| Tool calls | 12 | 15 |
| TaskSpace controls | 0 | 7 |
| Wall time | 15.34s | 32.37s |
| Request 2+ cache hit | 94.68% | 97.03% |
| Strict prefix | 6/6 | 15/15 |

配对双方均使用镜像
`sha256:55a8ac465c574efb57d8bd53f286812a77f41fd428de1c3b0b18b7c5165ee0ca`、
4 CPU、8 GiB memory、`/workspace` cwd 和 `/artifacts` 输出路径。Agent oracle mount、canary
泄漏、secret 精确命中和残留容器均为0。

## 4. Observability 断链修复

首轮容器执行已生成 rollout，但 exporter 仍调用 `Find-LatestRollout` 搜索宿主 Whale home，导致
control 可从 rollout 计数而 map 拓扑仍静默为0。修复后，exporter 优先使用 side artifact 中的
`rollout.jsonl`，仅在该文件不存在时回退原查找路径。

集成回归证据：
`target/r5-i2-observability-regression/count-call-stack/20260711-035335-892`。

| 项目 | 修复后结果 |
|---|---:|
| Rollout bytes | 23,184,714 |
| Rollout scan mode | full |
| Maps | 1 |
| Nodes | 5 |
| Edges | 4 |
| Open leaves | 0 |
| Ordinary tool before binding | false |
| Metrics warnings / taints | 0 / 0 |

该回归为 right-only 诊断样本，不参与 paired utility 聚合。它的22 requests、12 controls 和仍为
active 的根 task 是 J0-J4 的待优化基线，不是 I2 的失败，也不能被解释为 I2 收益。

## 5. 自动验证

```text
test-container-contract.ps1: passed
test-container-runtime.ps1: passed
test-container-benchmark-runner.ps1: passed
run-taskspace-benchmark.ps1 -PlanOnly: passed
count-call-stack paired Docker smoke: both solved
count-call-stack right-only observability regression: solved, 1 map / 5 nodes / 4 edges
```

## 6. 下一门禁

进入 R5-J0，冻结固定三节点拓扑和 provider capability contract。不得用减少节点、提示 Agent
粗化 Map 或 runtime 自动推进状态来降低请求数。
