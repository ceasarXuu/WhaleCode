# R5 Docker I0 基线

## 1. 结论

I0 已冻结 Docker benchmark contract、权限边界、资源配置、日志阈值和观测频率。该阶段只确定
机械运行边界，不改变 Agent、TaskSpace 或工具语义。

## 2. 固定配置

| 项目 | 冻结值 | 原因 |
|---|---:|---|
| base image | Ubuntu 24.04 digest `sha256:4fbb...2d90` | Whale 需要 glibc 2.39 |
| CPU | 4 | Standard/R5 同构且为并行工具留出空间 |
| memory | 8 GiB | 覆盖 debug Whale、pytest 和长上下文运行 |
| swap | 等于 memory | 禁止额外 swap 放大不可控抖动 |
| PIDs | 512 | 支持工具并行并限制失控进程树 |
| Docker log | `local`, 10 MiB × 3 | 常规日志充分，关键原始证据另写 `/artifacts` |
| stats interval | 5s | 本机 `docker stats --no-stream` 平均约 1.9s |
| workspace | `/workspace` | 消除 `/app` 和宿主长路径差异 |
| artifacts | `/artifacts` | 原始证据直接写回宿主 |

## 3. 校准证据

2026-07-11 对 `target/` 中 34,824 个 `*.log`/`*.jsonl` 文件统计：p50=259 B、p95=24,286 B、
p99=145,161 B、max=287,322,302 B。超大文件不能依赖 Docker logging driver，必须继续通过
bind mount 原样落盘。

对空闲 Ubuntu 容器执行十次 `docker stats --no-stream` 共耗时约 19.1s，因此 1s 采样会产生
明显观测干扰，正式值调整为 5s。

宿主 `fs.inotify.max_user_watches=65536`，观测时当前 UID 已使用约 65,394。该值进入 preflight
环境告警；Docker benchmark 不修改宿主 sysctl，也不把宿主资源异常解释成 Agent 失败。

## 4. 权限边界

| Role | workspace | artifacts | Whale | provider secret | hidden oracle |
|---|---|---|---|---|---|
| Agent | rw | rw | ro | ro | none |
| Validator | rw | rw | none | none | none |
| Oracle | ro | rw | none | none | ro |

API key 不进入 image、Dockerfile、build args、container env 或 artifacts；Agent 启动脚本只在容器
内部从只读 secret file 读取并导出到子进程。

## 5. 验证

`pwsh -NoProfile -File scripts/taskspace-benchmark/test-container-contract.ps1` 通过，覆盖 digest
固定、资源参数、日志参数、三角色权限矩阵及未固定 tag 的拒绝路径。
