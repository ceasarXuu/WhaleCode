# R5 统一 Docker Benchmark 与日志证据链计划

## 1. 元数据

- Created: 2026-07-10
- Updated: 2026-07-10
- Version: v0.0.5 build-R5 follow-up
- Status: Planned - explicitly deferred; do not execute until the user resumes this phase
- Owner / Responsible: WhaleCode benchmark harness
- Related Systems: benchmark runner、workspace、Whale CLI、public validator、hidden oracle、performance observation
- Related Links: `01-r5-phased-simplification-plan.md`、`12-r5-performance-observation-tool.md`
- Risk Level: High
- Plan Type: Full

## 2. 决策

后续 benchmark 收敛为 Docker-only：Agent、public validator 和 hidden oracle 都在容器中运行，宿主机只负责编排 Docker、注入临时 secret、保存 artifacts 和执行最终机械汇总。

迁移通过门禁后删除本机执行路径，不保留兼容分支或静默 fallback。Docker backend 不可用时，benchmark 必须在 preflight 阶段明确失败。

本计划当前只登记，不执行镜像构建、容器启动、代码迁移或样本重跑。

## 3. 当前事实

| 路径 | 当前状态 | 问题 |
|---|---|---|
| 自定义 sample Agent | 本机 `target/.../repo` 下的 Whale 子进程 | 继承宿主机 PATH、Conda 和依赖状态 |
| 自定义 public validator | 本机子进程 | `subscription-billing-repair` 已因本机 Miniconda 缺 pytest 失败 |
| hidden oracle | 本机隔离进程 | 隔离由 harness 维护，但执行环境仍依赖宿主机 |
| Terminal-Bench validator | 已有 Docker equivalent path | 只覆盖外部 validator，不是统一 Agent 执行 substrate |
| Docker timing | 已有 build/run/cleanup/cache 字段 | 没有统一覆盖所有 sample 和容器生命周期 |
| 日志 | stdout/stderr、metrics、events 分散存在 | 缺少统一 container id/image digest/phase/correlation 证据链 |

## 4. 目标与非目标

### 4.1 目标

| Goal | Expected Benefit | Verification |
|---|---|---|
| 所有正式 sample 使用 Docker-only 路径 | 消除宿主 Python、pytest、PATH 和 Conda 干扰 | 正式报告中 host executable path 命中为 0；container runtime coverage 100% |
| Standard/R5 使用相同 image digest 和资源配置 | 提高配对公平性和可复现性 | 每个 pair 的 image digest/resource profile equality=100% |
| 固定容器路径 | 避免 `/app`、长宿主路径和 side 路径差异影响 Agent | Agent cwd 固定 `/workspace`；artifact 固定 `/artifacts` |
| 建立完整日志证据链 | 能区分 build、create、preflight、agent、validator、oracle、collect、cleanup 失败 | 生命周期事件覆盖率100%，失败有稳定 reason code |
| 分离容器开销和 Agent 性能 | 防止镜像冷启动污染请求/模型收益判断 | build/pull/create/start/log/cleanup 与 agent wall 分项报告 |
| 删除本机 fallback | 避免两套执行语义长期漂移 | default-path scan 中 local agent/validator runner 为0 |

### 4.2 非目标

- 不通过容器约束 Agent 的语义动作或替 Agent 规划 Map。
- 不把 Docker 启动变快写成 R5 TaskSpace 智能收益。
- 不把 Agent、public validator 和 hidden oracle 放进同一安全域。
- 不在镜像中烘焙 API key、`.env.local` 或 hidden oracle。
- 不在本次文档变更中执行任何迁移。

## 5. 外部依据

1. [Docker build best practices](https://docs.docker.com/build/building/best-practices/)：使用可信基础镜像、digest 固定、ephemeral container 和 build cache。
2. [Docker logging drivers](https://docs.docker.com/engine/logging/configure/)：`local` driver 默认支持轮转并避免无界 `json-file` 占满磁盘。
3. [Docker container logs](https://docs.docker.com/reference/cli/docker/container/logs/)：使用 RFC3339Nano 时间戳和 details 获取可关联日志。
4. [Docker resource constraints](https://docs.docker.com/engine/containers/resource_constraints/)：CPU、memory 和 swap 默认不受限，正式对比必须显式记录或约束。
5. [Docker build secrets](https://docs.docker.com/build/building/secrets/)：禁止使用会持久化到镜像的 build args/env 传递构建 secret。
6. [OpenTelemetry Logs Data Model](https://opentelemetry.io/docs/specs/otel/logs/data-model/)：统一 timestamp、severity、trace/correlation 和 structured body 字段。

## 6. 目标架构

```text
Host orchestrator
  -> resolve/build image and record digest
  -> create Agent container (no oracle mount)
       /workspace  side repo rw
       /artifacts  side artifacts rw
       /run/secrets/deepseek_api_key  temporary ro
  -> collect Agent exit/inspect/logs/stats
  -> create public-validator container
       same image digest, /workspace, no hidden oracle
  -> create hidden-oracle container
       same runtime family, repo ro, private oracle ro
  -> collect all evidence
  -> remove containers
  -> generate performance-observation.json/md/events
```

Agent、public validator 和 hidden oracle 可以共享已验证的基础镜像或依赖层，但必须使用不同容器和不同 mount 权限。Standard/R5 每个 pair 必须使用完全相同的 image digest、workspace layout、secret policy、network policy 和 resource profile。

## 7. Artifact 与日志契约

每个 side 新增以下容器级 artifacts：

```text
container-runtime-manifest.json
container-lifecycle-events.jsonl
container-agent.stdout.log
container-agent.stderr.log
container-validator.stdout.log
container-validator.stderr.log
container-oracle.stdout.log
container-oracle.stderr.log
container-inspect-agent.json
container-inspect-validator.json
container-inspect-oracle.json
container-stats.jsonl
container-cleanup-result.json
```

### 7.1 必须字段

| Category | Fields |
|---|---|
| identity | `run_id/sample_id/pair_id/side/logical_mode` |
| container | `container_id/container_name/role/image_digest/docker_server_version` |
| execution | `phase/status/started_at/finished_at/duration_ms/exit_code/timeout` |
| workspace | `container_workdir/workspace_mount_mode/artifact_mount_mode` |
| resources | `cpu_limit/memory_limit/swap_policy/cpu_percent/memory_bytes/net_io/block_io/pids` |
| failure | `reason_code/failure_stage/cleanup_required/retryable` |
| correlation | `turn_id/provider_request_id/tool_call_id/trace_id`，不可用时明确为空 |

### 7.2 日志原则

- 容器 stdout/stderr 使用 per-container `local` logging driver 和有界轮转；具体容量由 I0 根据现有 rollout/log p99 冻结。
- 容器删除前执行 `docker logs --timestamps`，把可读日志复制到 side artifacts。
- 关键结构化事件直接写入 bind-mounted `/artifacts`，不能只依赖 logging driver 缓冲。
- Agent JSONL、provider wire/cache trace、Map event 继续使用现有原始 schema，不重新摘要。
- 默认不记录完整环境变量、secret mount 内容、provider 请求正文或用户代码正文。
- `docker stats` 采样频率和观测开销必须在 I0 校准，并对 Standard/R5 一致。

### 7.3 Change-chain Logging Matrix

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation | Level | Consumer |
|---|---|---|---|---|---|---|---|
| image resolve/build | resolved/built | `container.image_ready` | `container.image_failed` | `reason_code` | `run_id/image_digest` | info/error | harness/operator |
| container create | created | `container.created` | `container.create_failed` | `reason_code` | `pair_id/side/container_id` | info/error | harness |
| environment preflight | validated | `container.preflight_passed` | `container.preflight_failed` | `missing_capability` | `container_id/image_digest` | info/error | benchmark gate |
| Agent execution | started/completed | `container.agent_completed` | timeout/nonzero/interrupted | `exit_reason` | `container_id/turn_id` | info/warn | benchmark/runtime |
| public validation | started/completed | `container.validation_completed` | infra/test failure | `failure_class` | `container_id/pair_id` | info/warn | correctness gate |
| hidden oracle | started/completed | `container.oracle_completed` | isolation/runtime failure | `failure_class` | `container_id/pair_id` | info/error | evidence gate |
| log collection | collected | `container.logs_collected` | missing/truncated | `missing_stream` | `container_id` | info/error | performance observer |
| cleanup | removed | `container.cleanup_completed` | inspect/rm failure | `cleanup_reason` | `container_id/run_id` | info/error | harness health |

## 8. 分阶段执行计划

### R5-I0：现状、契约和基线冻结

**Entry:** 用户明确恢复本计划；当前本机和 Terminal-Bench Docker 路径可审计。

**Tasks:**

1. 画出 Agent/public/oracle 当前进程和权限边界。
2. 冻结 container runtime manifest、lifecycle event 和 reason-code schema。
3. 统计当前日志体积、Docker stats 采样开销、镜像构建时间和现有资源使用。
4. 冻结 CPU/memory/swap、日志轮转和 stats interval；未知值不得带入 I1。
5. 建立 secret、oracle、workspace、artifact mount 权限矩阵。

**Exit:** schema fixture、权限矩阵、资源/日志基线和阈值全部有独立测试；不需要 I1 代码证明 I0。

**Fallback:** 继续停留在本机诊断模式，不进入容器实现。

### R5-I1：日志优先的容器 substrate

**Entry:** I0 100%完成。

**Tasks:**

1. 实现统一 image resolve/build、digest、label 和 preflight 模块。
2. 先实现 lifecycle events、inspect、timestamped logs、stats 和 cleanup，再接 Agent。
3. 固定 `/workspace`、`/artifacts` 和非 oracle secret mount。
4. 基础镜像和依赖版本使用 digest/lock 固定；Agent 运行期不得依赖宿主 Python/pytest。
5. API key 从 `.env.local` 读取后写入权限受限的临时 secret file，只读挂载，结束后机械清理；不得写入 Dockerfile、image layer、inspectable Config.Env 或 artifact。

**Exit:** `true`/preflight/forced failure/timeout/cleanup fixtures 均产生完整日志链；secret scan 为0；容器不存在残留。

**Fallback:** git revert I1；不增加本机/Docker运行时 fallback 分支。

### R5-I2：Agent、Validator、Oracle 全链迁移

**Entry:** I1 日志和安全门禁通过。

**Tasks:**

1. 让 Standard/R5 Agent 都通过同一 container runner，treatment delta 仍只有 TaskSpace 开关。
2. public validator 迁入独立容器并在 Agent 前执行依赖 preflight。
3. hidden oracle 迁入私有 mount 容器，Agent container 不得获得 oracle path、image layer 或 mount。
4. provider/cache/Map artifacts 通过 `/artifacts` 原样写回。
5. 所有 container phase 的时间与 Agent/model/validation 时间分账。

**Exit:** `count-call-stack` 单次 paired smoke 双方完成；image/resource/path parity 100%；oracle leak test 通过；日志覆盖100%。

**Fallback:** revert I2；保持 I1 substrate 未接 default path，不做动态本机 fallback。

### R5-I3：等价性、性能与复杂样本门禁

**Entry:** I2 单样本正确性通过。

**Tasks:**

1. 用相同 revision 分别产生历史本机诊断和 Docker candidate，验证 patch、validation、provider/cache/Map artifact 等价。
2. Docker 下执行 `count-call-stack` Standard/R5 交替三轮。
3. Docker 下完整配对执行 `subscription-billing-repair` 和一个依赖/并行复杂样本。
4. 验证 pytest 等依赖 preflight 在 Agent 前通过，运行期不再搜索宿主环境。
5. 用 performance observer 分离 container setup 与 Agent 性能，量化日志/stats 观测开销。

**Exit:** correctness 无回退；host environment failure=0；paired image/resource parity=100%；container lifecycle/log coverage=100%；观测开销低于 I0 冻结阈值。

**Fallback:** 暂停 cutover并修复容器 substrate；不得用本机结果替代 Docker 正式收益证据。

### R5-I4：Docker-only 切换与本机路径删除

**Entry:** I3 100%完成。

**Tasks:**

1. Docker runner 成为唯一 benchmark execution path。
2. 删除本机 Agent、public validator 和 hidden oracle 执行分支及对应兼容测试。
3. Docker unavailable、image mismatch、preflight failure 都在 Agent 启动前明确失败。
4. CI/开发命令和 performance observer 文档切换为 Docker-only。
5. 扫描 local fallback、host Python/pytest、未固定 image tag 和 secret 泄漏。

**Exit:** production call graph 不可达本机 runner；正式样本全部 `container_runtime_coverage=100%`；git clean；文档和日志 schema 一致。

**Rollback:** 只允许 git revert 到 I3 前版本，不保留运行时双路径或自动 fallback。

## 9. Phase Gate Matrix

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Required Before Next | Decision |
|---|---|---|---|---|---|
| I0 | schema/permission/resource baseline fixtures | 不依赖 container runner | I0 contract artifacts | 100% | pause until user resumes |
| I1 | lifecycle/secret/failure/cleanup smoke | 不依赖 Agent migration | substrate logs and scans | 100% | proceed I2 |
| I2 | one paired real sample and oracle isolation | 不依赖 complex benchmark | pair report and container manifests | 100% | proceed I3 |
| I3 | controlled repeats and complex pairs | 不依赖 default switch | performance observation and parity report | 100% | proceed I4 |
| I4 | call-graph scan and Docker-only real run | 无 | default-path and clean-tree evidence | 100% | close |

## 10. Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock Exposure | Status |
|---|---|---|---|---|---|---|---|
| container contract | 固定 identity/path/resource/secret schema | `scripts/taskspace-benchmark/lib/` 新容器模块 | benchmark CLI | schema fixtures | runtime manifest | test-only fixture | planned |
| image/preflight | digest 和依赖可复现 | Dockerfile/build module | run preflight | image/preflight tests | image lifecycle events | fake Docker blocks completion | planned |
| log collector | 失败前后日志可恢复 | container log/stats collector | every container phase | timeout/truncation/rotation tests | lifecycle events/log artifacts | fake Docker then real smoke | planned |
| Agent runner | Standard/R5 同一容器路径 | benchmark side executor | `run-taskspace-benchmark.ps1` | paired smoke | agent container manifest | none at exit | planned |
| validator/oracle | 私有边界容器化 | oracle/validation runner | post-Agent validation | leak/isolation tests | validator/oracle events | none at exit | planned |
| Docker-only cutover | 本机路径不可达 | bootstrap/runner cleanup | all benchmark commands | call graph + real runs | runtime coverage | none | planned |

## 11. 风险与缓解

| Risk | Probability | Impact | Trigger | Mitigation | Fallback |
|---|---:|---:|---|---|---|
| Docker 冷启动污染性能 | High | Medium | setup time 混入 Agent wall | 全阶段分账，镜像预热，固定 cache key | I3 不进入 cutover |
| 日志轮转或 non-blocking 丢失证据 | Medium | High | stream 不完整 | 关键事件写 bind mount；删除前 logs 导出；阻止静默截断 | 标记 run invalid |
| stats 采样改变性能 | Medium | Medium | observer overhead 超阈值 | I0 校准并两侧一致 | 降低采样频率后重验 |
| API key 进入 image/inspect/log | Low | Critical | secret scanner 命中 | 临时只读 secret mount、redaction、构建期禁用 env/arg | 立即销毁 artifact/轮换 key/阻止 phase |
| hidden oracle 暴露给 Agent | Low | Critical | mount/path/layer leak | 独立容器与 image/mount 审计 | run invalid，阻止 cutover |
| 多语言 sample 无法共用单镜像 | High | Medium | 依赖冲突 | 统一 contract，不强制单镜像；base + fixture layer | I0 调整 image strategy |
| Docker backend 不可用 | Medium | High | preflight failure | 启动前明确报告 backend 状态 | 阻止运行，不回退本机 |
| 容器路径改变任务语义 | Medium | High | `/app`/`/workspace` 错误历史 | 固定路径，Standard/R5 同构，路径 artifact 扫描 | 暂停 I2 |

## 12. Open Questions

| Question | Owner | Resolution Gate |
|---|---|---|
| CPU、memory、swap 和 stats interval 的具体值 | benchmark owner | I0 |
| 使用单一多语言镜像还是 base + sample layer | benchmark owner | I0 |
| provider egress 与依赖下载的网络策略 | runtime/security | I0 |
| 日志轮转容量和 artifact 保留周期 | benchmark owner | I0 |
| CI Docker backend 和镜像缓存位置 | CI owner | I1 |

## 13. 当前暂停点

本计划已完成设计登记，当前停在 I0 之前。除文档和计划索引外，不应产生 Dockerfile、容器 runner、日志 collector、镜像或新 benchmark run。
