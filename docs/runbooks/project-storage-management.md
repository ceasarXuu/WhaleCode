# 项目存储管理

## 目标与边界

本 runbook 将存储分为两类，禁止混合处理：

1. 应用构建缓存：Cargo profile 目录、Node 依赖、Docker 镜像与 Docker build cache。默认保留，只观测，不由 benchmark 清理命令删除。
2. benchmark 运行产物：run root、workspace 副本、日志、rollout、冻结二进制和自测临时目录。受容量门禁和显式保留策略管理。

`scripts/taskspace-benchmark/cleanup-taskspace-artifacts.ps1` 只扫描仓库根
`target/` 的直接子项，并永久保护以下 Cargo 目录：

```text
debug release dev dev-small dist ci-test doc package
```

它不会扫描或删除 `third_party/codex-cli/codex-rs/target`，也不会调用
`docker system prune`、`docker builder prune` 或删除镜像。

## 容量门禁

base benchmark runner 在运行前和每个 pair 完成后写入
`artifact-storage-<stage>.json`。默认硬限制为：

| 范围 | 默认限制 | 环境变量 |
|---|---:|---|
| 单个 run root | 24GiB | `TASKSPACE_MAX_RUN_ARTIFACT_GIB` / `TASKSPACE_MAX_RUN_ARTIFACT_BYTES` |
| 仓库 benchmark 产物总量 | 64GiB | `TASKSPACE_MAX_REPO_ARTIFACT_GIB` / `TASKSPACE_MAX_REPO_ARTIFACT_BYTES` |

仓库总量统计会排除上表中的 Cargo profile 目录。以下情况按
`invalid_harness` 失败，不能静默继续：

- `benchmark_run_artifact_limit_exceeded`
- `benchmark_repository_artifact_limit_exceeded`
- `benchmark_nested_build_cache_detected`
- `benchmark_artifact_scan_failed`
- `benchmark_artifact_limit_invalid`

其中 `benchmark_nested_build_cache_detected` 专门阻止 run root 内出现
`third_party/codex-cli/codex-rs/target` 副本。这是本次几十 GiB 重复复制的直接工程门禁。

## 清理流程

先生成计划，不删除：

```bash
pwsh -NoProfile -File scripts/taskspace-benchmark/cleanup-taskspace-artifacts.ps1 \
  -MinimumAgeDays 7
```

需要保留当前实验证据时，显式传入根目录名：

```powershell
& scripts/taskspace-benchmark/cleanup-taskspace-artifacts.ps1 `
  -MinimumAgeDays 7 `
  -KeepName @("r7-five-layer-eval-data", "r7-five-layer-matrix")
```

审阅 `.storage-reports/cleanup-*.json` 后，才执行删除：

```powershell
& scripts/taskspace-benchmark/cleanup-taskspace-artifacts.ps1 `
  -MinimumAgeDays 7 `
  -KeepName @("r7-five-layer-eval-data", "r7-five-layer-matrix") `
  -Apply
```

`-Apply` 仍会逐项记录 `removed` 或 `failed`。Docker 生成的 root-owned
目录如果权限不足会明确失败，不会被当作成功。处理这类目录时，只允许把仓库
`target/` 挂载到临时容器中定界清理，禁止挂载整个 home 或执行全局 prune。

## 构建缓存健康

Cargo 不会自动限制 `target` 的容量。当前项目的普通 `cargo test` 使用默认
test profile；`[profile.ci-test]` 只有显式传入 `--profile ci-test` 才生效。
因此频繁修改大型 crate 时，历史 hash、测试可执行文件和 incremental session
会持续保留。

构建缓存治理必须独立决策，不能借 benchmark 清理顺便删除。优先方向为：

1. 将日常测试放入独立、可回收的 test profile/target；应用开发构建继续使用稳定缓存。
2. 对测试 profile 评估关闭 incremental 或降低 debug info，分别测量构建时间和容量收益。
3. 为构建缓存增加只读容量报告；任何自动清理都必须显式启用并记录影响范围。
4. 禁止用完整目录复制冻结版本；优先使用 Git commit、manifest、hash 和可重建命令。

2026-07-23 曾在用户明确授权后单独删除 `debug/incremental`，实际释放约
147GiB，并保留 `deps` 和现有二进制。该操作不是常规自动维护策略：incremental
会在后续编译中重新生成，长期方案仍需通过测试 profile 隔离和构建耗时/容量 A/B
验证后确定。

自测生成物默认写入唯一临时目录，成功后立即清理。测试失败时可以保留诊断目录；
正式 benchmark 证据仍写入显式 run root。禁止把每次成功自测移动到 `*-complete`
或 `.bak-*` 目录，这类“归档”没有保留上限，会绕过仓库 `target/` 的容量门禁。

## 验证

```bash
pwsh -NoProfile -File scripts/taskspace-benchmark/test-artifact-storage.ps1
pwsh -NoProfile -File scripts/taskspace-benchmark/test-r7-five-layer-matrix-harness.ps1
```

测试覆盖构建缓存排除、run/repository 超限、嵌套 Cargo target、dry-run 无副作用、
显式 apply 和保留项。
