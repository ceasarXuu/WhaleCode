# R7 存储异常清理与防复发

## 结论

2026-07-23 开发前审计发现仓库占用约 386GiB。异常不是单个日志增长，主要由
benchmark 历史运行目录反复保存源码树、Cargo target、冻结二进制和容器 artifacts
造成。已在不删除应用构建缓存、Docker 镜像和 Docker build cache 的前提下完成首轮
清理，并落地 benchmark 容量门禁。

## 清理证据

| 项目 | 清理前 | 清理后 | 处理 |
|---|---:|---:|---|
| 仓库根 `target/` | 145,008,549,888 B | 2,847,064,064 B | 回收 142,161,485,824 B |
| Cargo 主缓存 | 268,433,444,864 B | 268,433,444,864 B | 完整保留 |
| stopped `whale-*` 容器 | 113 / 2.158GB | 0 / 0B | 删除运行容器 |
| Docker build cache | 5.08GB | 5.08GB | 完整保留 |
| Docker images | 24 / 8.611GB | 24 / 8.611GB | 完整保留 |
| `/home` 已用空间 | 约 670GB | 约 538GB | 可用空间增至约 1014GB |

根 `target/` 保留：

- `debug`：根 workspace 构建缓存；
- `.rustc_info.json`；
- `r7-five-layer-eval-data`：最新四臂 repeat-3 原始证据；
- `r7-five-layer-matrix`：对应报告和 trace 分析。

## 直接根因

旧产物中存在以下可再生大目录：

| 路径 | 大小 | 原因 |
|---|---:|---|
| `target/r7-toolchain` | 42GiB | 多次集成/回滚演练复制 4.6GiB `third_party` 和局部 target |
| `target/r5-map-compression` | 26GiB | 冻结源码含 17GiB 构建树，并重复保存约 1GiB 二进制 |
| `target/r4-e3-formal-p0-*` | 13GiB | 历史正式矩阵及 workspace/artifacts |
| `target/r6-phase-e` | 11GiB | R5 冻结源码副本约 10GiB |

benchmark runner 原有磁盘门禁只检查“剩余空间是否低于 20GiB”，不能识别产物
总量不断增长。在 1.6TiB 分区上，即使仓库已增长到近 400GiB，门禁仍会通过。

## Cargo 250GiB 专项结论

该目录本轮按用户边界未删除，但其规模不健康：

| 组成 | 实占 | 观测 |
|---|---:|---|
| `debug/incremental` | 161,601,531,904 B | 2,515 个 crate 目录，4,044 个 session |
| `debug/deps` | 102,540,595,200 B | 26,855 个文件 |
| `debug/build` | 2,636,730,368 B | build-script 输出 |
| `debug/examples` | 1,159,733,248 B | 示例构建 |

约 228GB 超过一天未更新，158GB 超过三天，124GB 超过七天。主要放大器是：

1. 普通 `cargo test` 未使用已定义的 `ci-test` profile；
2. 默认开发/测试编译启用 incremental；
3. `codex-core`、app-server protocol 和 TUI 测试目标单体很大；
4. Cargo 为源码、target kind 和 feature 变化生成新 hash，但没有自动容量淘汰。

这说明“构建缓存”内部还需区分当前热缓存与历史编译图。该问题不能通过删除
benchmark artifacts 根治，后续必须单独验证测试缓存隔离方案。

## 已落地防线

1. 新增共享 `artifact-storage.ps1`，统一容量口径和稳定错误码。
2. base benchmark runner 在 run preflight 和每个 pair 完成后检查容量并写 JSON 事件。
3. 单 run 默认 24GiB，仓库 benchmark artifacts 默认 64GiB；构建 profile 永久排除。
4. run root 内嵌套 `third_party/codex-cli/codex-rs/target` 立即失败。
5. 新增默认 dry-run 的显式清理工具，`-Apply` 才执行删除。
6. 活跃 self-test 改为唯一临时目录且成功后清理，不再生成无界 `*-complete` 和
   `.bak-*` 归档。

详细操作见 [项目存储管理](../../runbooks/project-storage-management.md)。
