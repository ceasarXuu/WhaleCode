# TaskSpace Benchmark Harness 工程实施计划

日期：2026-05-31

## 目标

实现 TaskSpace paired benchmark harness 的最小可运行闭环，用真实 Whale CLI 对同一场景分别运行 `standard` 与 `taskspace`，并生成可复核的 paired report。

第一阶段目标不是证明 TaskSpace 已经有真实复杂任务收益，而是建立可信测试基础设施：

- 同一 prompt、fixture、model、权限、环境。
- 唯一 treatment delta 是 `--taskspace` 及其必然 runtime 行为。
- hidden oracle 不暴露给 agent。
- 用户 prompt 按真实用户叙事，不泄漏 TaskSpace 内部概念。
- 失败不被覆盖，artifact 可追溯。
- L1/L2 两个场景能稳定产生 pair report。

对应证据等级：

```text
MVP 完成后：E1 机制回归 + E2 候选证据
只有 paired 对照、重复运行、统一 oracle 和变量控制都通过后，
才能把某个场景标成 E2 Paired Utility。
```

证据等级必须由 harness 计算，不能信任 `scenario.json` 的声明。

```text
scenario.evidence_target = 期望目标
reported_evidence_level = harness 根据实际证据计算出的等级
```

E2 最低门禁：

- `Repeats >= 3`。
- 所有 pair 都满足 `invalid_pair = false`。
- 所有 pair 都满足 `invalid_prompt = false`。
- prompt/fixture/whale/model/permission/cwd/timeout checks 全部通过。
- provider/model 参数可观测，或差异被明确记录为 allowed delta。
- manual review required 的 prompt 已闭环并写入 report。
- hidden oracle isolation 必须为 hard sandbox；accepted soft isolation 必须明确降级为 E1/E2-candidate。
- hidden oracle/public validation/TaskSpace structural oracle 结果完整。

如果任一条件不满足，report 只能标为 E1 或 E2-candidate，不能进入 E2 utility aggregate。

## 非目标

- 不接入 Terminal-Bench。
- 不做 L4 多轮/压缩 runner。
- 不实现完整统计显著性分析。
- 不实现复杂 NLP prompt classifier。
- 不重构现有 TaskSpace runtime。
- 不用 benchmark 结果宣称真实世界收益。

## 设计输入

本计划以以下文档为准：

- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`
- `docs/plans/2026-05-31-basemap-decomposition-methodology.md`
- `docs/plans/2026-05-30-taskspace-e2e-correction.md`

现有 E1 脚本可作为迁移来源：

- `scripts/run-action-map-natural-multi-agent-e2e.ps1`
- `scripts/run-action-map-growth-health-e2e.ps1`
- `scripts/action-map-graph-health-lib.ps1`
- `scripts/action-map-real-user-e2e-lib.ps1`
- `scripts/export-action-map-observability.ps1`

复用清单：

| 现有脚本 | 复用方式 |
|---|---|
| `scripts/action-map-graph-health-lib.ps1` | 直接复用图健康计算，避免重写 edge/order/open leaf 判断 |
| `scripts/action-map-observability-lib.ps1` | 复用 observability 聚合逻辑 |
| `scripts/action-map-real-user-e2e-lib.ps1` | 复用真实进程执行、tool failure 分类、prompt 检查辅助 |
| `scripts/export-action-map-observability.ps1` | 作为 wrapper 调用，不重写 rollout 到 observability 的导出路径 |
| `scripts/test-action-map-graph-health.ps1` | 作为图健康库回归参考 |
| `scripts/test-action-map-observability-lib.ps1` | 作为 observability 聚合回归参考 |
| `scripts/run-action-map-natural-multi-agent-e2e.ps1` | 迁移 prompt、oracle、order-pipeline fixture 思路，不直接复制整脚本结构 |
| `scripts/run-action-map-growth-health-e2e.ps1` | 迁移 graph health 断言与 hidden oracle 思路 |

旧 E1 runner 保留，不在第一阶段删除或改名。新 harness 先并行存在，稳定后再考虑迁移。

## 第一阶段范围

### 场景

先实现两个场景：

| 场景 | 层级 | 目的 |
|---|---|---|
| `single-file-fast-fix` | L1 | 验证 TaskSpace 不拖累简单任务 |
| `order-pipeline-growth` | L2 | 迁移现有 order-pipeline E2E，验证 paired 对照和结构 oracle |

### Runner

先实现单命令入口：

```powershell
.\scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 `
  -Scenario single-file-fast-fix `
  -Repeats 1 `
  -Model deepseek-v4-flash
```

MVP 支持：

- 单 scenario。
- `Repeats` 参数，默认 1。
- 自动运行 standard + taskspace pair。
- 输出 `pair-report.md`。
- 生成 `run-summary.md`，只汇总当前 pair 的 artifact、校验、oracle 结果和证据等级；不计算 utility aggregate。

暂不支持：

- 多 scenario 批量矩阵。
- L4 多 turn。
- 外部 benchmark adapter。
- 强制上下文压缩。

## 目录结构

新增：

```text
scripts/
  taskspace-benchmark/
    run-taskspace-benchmark.ps1
    lib/
      scenario-manifest.ps1
      prompt-guard.ps1
      workspace.ps1
      oracle-runner.ps1
      metrics-extractor.ps1
      pair-report.ps1
      aggregate-report.ps1

benchmarks/
  taskspace/
    scenarios/
      single-file-fast-fix/
        scenario.json
        prompt.txt
        fixture/
        private-oracle/
      order-pipeline-growth/
        scenario.json
        prompt.txt
        fixture/
        private-oracle/
```

输出：

```text
target/taskspace-benchmark/<scenario>/<run_id>/
  manifest.resolved.json
  logical-mode-map.json
  left/
    repo/
    artifacts/
  right/
    repo/
    artifacts/
  reviewer-only/
    private-oracle/
  pair-report.md
  run-summary.md
  aggregate-report.md      # MVP+1 起生成；MVP 不计算 utility aggregate
```

## 模块职责

### `scenario-manifest.ps1`

职责：

- 读取 `scenario.json`。
- 校验必填字段。
- 解析 prompt、fixture、private oracle 路径。
- 校验 `narrative_contract`、`mode_delta_contract`。
- 输出 normalized manifest object。

MVP 必填字段：

```json
{
  "id": "...",
  "level": "L1|L2",
  "evidence_target": "E1|E2",
  "prompt_file": "prompt.txt",
  "fixture_dir": "fixture",
  "narrative_contract": {},
  "mode_delta_contract": {},
  "oracle": {},
  "expected": {},
  "thresholds": {}
}
```

### `prompt-guard.ps1`

职责：

- 执行用户叙事硬约束检查。
- hard internal token 命中直接 `invalid_prompt = true`。
- context-sensitive term 命中时输出 `manual_review_required` 或 allowed reason。
- 输出 guard result JSON。

MVP 实现策略：

- hard token 用正则匹配。
- context-sensitive term 只做命中记录，不自动 invalid。
- 如果命中 context-sensitive term 且没有 allowlist reason，设置 `manual_review_required = true`。
- harness 默认不让 `manual_review_required` 进入 E2 aggregate，但可以保留 artifact。

必须有 self-test 覆盖：

- `taskspace`
- `subagent`
- “请并行派多个 agent”
- `Node.js`
- `source map`
- `parallel tests`
- `performance benchmark`

### `workspace.ps1`

职责：

- 创建 run directory。
- 复制 fixture 到 `left/repo`、`right/repo`。
- 初始化 git baseline。
- 计算 fixture checksum。
- 生成 `logical-mode-map.json`。
- 复制 private oracle 到 `reviewer-only/private-oracle`，不进入 repo。

关键约束：

- agent 可见 cwd 只能是 `left/repo` 或 `right/repo`。
- 不出现 `standard/repo`、`taskspace/repo`。
- 两侧初始 git diff 必须为空。

### `oracle-runner.ps1`

职责：

- 运行 public validation。
- 运行 hidden oracle。
- 写 stdout/stderr/exit code。
- 记录 oracle sha256。

关键约束：

- hidden oracle 从 repo 外执行。
- hidden oracle 脚本不复制到 agent artifacts。
- 如果发现 repo 内存在 private oracle 文件，标记 `oracle_isolation_failure = true`。
- 记录 `oracle_isolation_level`：
  - `hard_sandbox`：agent 工具无法访问 private oracle 路径。
  - `soft_denylist`：当前无法硬隔离，只能通过路径扫描和 denylist 检查。
  - `failed`：发现 agent 可见路径、JSONL、stderr、diff 或 tool call 触达 private oracle。
- 使用 `--dangerously-bypass-approvals-and-sandbox` 时，默认不能声明 `hard_sandbox`。
- `soft_denylist` 只能产出 E1 或 E2-candidate；不得进入 E2 utility aggregate。
- 用户显式接受 soft isolation 风险时，只能在 report 中标注 `accepted_soft_isolation = true`，或进入单独的 exploratory aggregate；不得提升为 E2。
- `failed` 必须排除出 utility aggregate。

soft denylist 扫描范围：

- `whale-exec.jsonl`
- stderr/stdout
- last message
- git diff
- tool call paths
- repo 内文件列表
- artifact 内可见路径

扫描目标：

- `private-oracle`
- `reviewer-only`
- hidden oracle 绝对路径
- source scenario 的 private oracle 路径

### `metrics-extractor.ps1`

职责：

- 从 `whale-exec.jsonl` 抽取通用指标。
- 从 git diff 抽取 changed paths。
- 从 observability JSON 抽取 TaskSpace 指标。
- 检查 allowed/forbidden modified paths。
- 输出 `metrics.json`。

MVP 通用指标：

```json
{
  "mode": "left|right",
  "logical_mode": "standard|taskspace",
  "exec_exit_code": 0,
  "public_validation_exit_code": 0,
  "hidden_oracle_exit_code": 0,
  "wall_time_ms": 0,
  "tool_call_count": 0,
  "changed_paths": [],
  "business_success": true,
  "invalid_prompt": false,
  "invalid_pair": false,
  "harness_failure": false
}
```

MVP TaskSpace 指标：

```json
{
  "maps": 0,
  "nodes": 0,
  "edges": 0,
  "edge_order_violations": 0,
  "spawn_agent_calls": 0,
  "subagent_results": 0,
  "open_leaf_nodes": 0,
  "ordinary_before_binding": false
}
```

### `pair-report.ps1`

职责：

- 汇总 left/right metrics。
- 显示 logical mode mapping。
- 显示 variable control 检查。
- 显示 prompt guard。
- 显示 public/hidden oracle。
- 显示 TaskSpace structural health。
- 给出 failure classification。

报告必须明确：

- `valid_pair: true|false`
- `included_in_utility_aggregate: true|false`
- 如果排除，说明原因。

### `aggregate-report.ps1`

职责：

- 汇总同 scenario 多次 repeats。
- 区分 `all pairs`、`valid utility pairs`、`excluded pairs`。
- 不做强统计显著性声明。
- MVP 阶段不调用；MVP+1 起作为 utility aggregate 的唯一入口。

MVP 聚合：

- business success rate。
- hidden oracle pass rate。
- median wall time。
- median tool calls。
- taskspace graph health pass rate。

## 执行流程

```text
run-taskspace-benchmark
  -> load scenario manifest
  -> run prompt guard
  -> create run id
  -> create left/right workspaces
  -> resolve logical mode map
  -> write manifest.resolved.json
  -> execute left
  -> execute right
  -> run public validation for both
  -> run hidden oracle for both
  -> export taskspace observability
  -> extract metrics
  -> compare variable control
  -> write pair-report
  -> write run-summary
  -> update aggregate-report when aggregate is enabled
```

## Whale 执行命令

standard logical mode：

```powershell
whale exec --json `
  -m <model> `
  -C <left-or-right-repo> `
  --dangerously-bypass-approvals-and-sandbox `
  --output-last-message <path> `
  -
```

taskspace logical mode：

```powershell
whale exec --json `
  --taskspace `
  -m <model> `
  -C <left-or-right-repo> `
  --dangerously-bypass-approvals-and-sandbox `
  --output-last-message <path> `
  -
```

唯一允许差异：

- `--taskspace`
- 由 `--taskspace` 引发的 TaskSpace runtime 行为、结构化工具和 observability。

## 变量控制

MVP 必须比较：

| 变量 | 检查 |
|---|---|
| prompt | sha256 相同 |
| fixture | sha256 相同 |
| whale binary | path/version/sha256 相同 |
| model | name 相同 |
| permissions | 命令参数相同 |
| cwd shape | left/right 中性路径 |
| timeout | 相同 |
| public validation | 相同 |
| hidden oracle | sha256 相同 |

暂时记录但不阻塞：

- provider default temperature。
- top_p。
- max output。
- reasoning effort。
- service tier。

如果当前 CLI 无法稳定读出这些 provider 参数，`manifest.resolved.json` 写 `provider-default-or-unknown`，pair report 标记为 `model_param_observability_gap`，但不阻止 E1。进入 E2 前必须解决。

## Evidence Gate

`reported_evidence_level` 由 harness 计算：

| 条件 | reported_evidence_level |
|---|---|
| 单次真实 run，artifact 完整但无 paired 对照 | E0 |
| paired run 完整，但 repeats < 3 或 provider/oracle isolation 存在 gap | E1 或 E2-candidate |
| repeats >= 3，变量控制通过，hard oracle isolation，manual review 闭环 | E2 |
| 外部 benchmark/历史真实 corpus + E2 条件 + 人工复核 | E3 |

禁止：

- `scenario.json` 直接决定最终证据等级。
- 单次 `Repeats 1` 标为 E2。
- provider 参数 unknown 时进入 E2。
- oracle isolation 为 `soft_denylist` 时进入 E2。
- manual review required 未闭环时进入 E2。

pair report 必须显示：

```text
requested_evidence_target: E2
reported_evidence_level: E1
evidence_gate_failures:
  - repeats_lt_3
  - provider_params_unknown
  - oracle_isolation_soft_denylist
```

## 首批场景设计

### `single-file-fast-fix`

目标：

- L1 简单任务不应被 TaskSpace 过度拆解。

fixture：

```text
src/tax_calc.py
tests/test_tax_calc.py
README.md
pyproject.toml
```

故障：

- 一个函数四舍五入错误或边界条件错误。

用户 prompt：

```text
这个小项目里有一个税费计算测试失败。请先看一下 README 和相关测试，修复实现，最后跑测试确认。
```

期望：

- standard/taskspace 都通过 hidden oracle。
- TaskSpace 不 spawn subagent。
- TaskSpace node count 不超过 3 到 4。
- wall time/tool calls 不超过 standard 的 1.5 倍，超出标记 cost regression。

### `order-pipeline-growth`

目标：

- 迁移现有 order-pipeline E2E。
- 验证多文件事实面、错误测试预期、实现 bug 和 TaskSpace graph health。

fixture：

```text
src/order_pipeline/parser.py
src/order_pipeline/pricing.py
src/order_pipeline/invoice.py
tests/test_parser.py
tests/test_pricing.py
tests/test_invoice.py
README.md
pyproject.toml
```

用户 prompt：

```text
我接手了这个 order-pipeline 小项目，parser、pricing discount、shipping、invoice total 看起来互相关联，有些测试可能和 README 不一致。请先检查 README、测试和实现，区分产品规则和错误预期，再完成修改。修改前先跑当前测试，修改后再跑相关测试。
```

注意：

- prompt 不提 taskspace、map、node、subagent、parallel。
- 如果需要多个调查轨道，由 TaskSpace 方法论和主 agent 自己决定。

期望：

- public validation pass。
- hidden oracle pass。
- taskspace 有 implementation -> validation 依赖。
- 如果产生 subagent，result 必须写回 node。
- 不要求 standard 有图结构。

## Harness 自测

新增脚本：

```text
scripts/taskspace-benchmark/test-harness.ps1
```

MVP 自测：

| 自测 | 目的 |
|---|---|
| manifest validation | 缺必填字段时报错 |
| prompt hard-token invalid | `taskspace/subagent/spawn_agent` 命中 invalid |
| prompt false-positive allowed | `Node.js/source map/parallel tests` 不自动 invalid |
| left/right isolation | agent cwd 不含 standard/taskspace |
| hidden oracle isolation | repo 内不存在 private oracle |
| invalid pair exclusion | prompt checksum 不同时 excluded |
| evidence-gate self-test | `Repeats 1` + `evidence_target=E2` 时降级为 E1/E2-candidate |
| provider-param observability gap | provider 参数 unknown 时不得进入 E2 |
| oracle path leak test | 构造读取 `private-oracle/reviewer-only` 路径，必须 failed 或排除 |
| soft-denylist accepted-risk test | `soft_denylist` 即使被用户显式接受，也不得进入 E2 utility aggregate |
| run-order alternation test | repeats 中 odd/even logical mapping 交替 |
| manual-review persistence test | context-sensitive 需要人工复核时，结论必须写入 pair report |

## 验收标准

MVP 完成条件：

- `single-file-fast-fix` 能生成完整 pair report。
- `single-file-fast-fix` 能生成 `run-summary.md`，但不生成 utility aggregate。
- left/right artifact 完整。
- prompt guard 输出 JSON。
- manifest.resolved.json 包含 checksum、mode mapping、whale sha256。
- oracle isolation level 被记录；如果不是 hard sandbox，report 不能标 E2。
- single-file 场景有 public validation 和 hidden oracle 结果。
- harness self-test 通过。

MVP+1 完成条件：

- `order-pipeline-growth` 能生成完整 pair report。
- taskspace 路径有 observability artifact。
- aggregate report 区分 valid/excluded pairs；该 report 是 utility aggregate 的第一个版本。

MVP+2 完成条件：

- `-Repeats 3` 能生成 3 个 pair。
- odd/even logical mapping 交替。
- aggregate report 可以计算 E2 gate，但只有满足所有门禁时才标 E2。

## 风险与处理

| 风险 | 处理 |
|---|---|
| whale exec 参数在当前版本不稳定 | 先复用现有 E2E 的调用方式 |
| taskspace observability export 路径变动 | 从现有脚本抽象 wrapper，不重写逻辑 |
| provider 参数不可观测 | E1 记录 gap，E2 前补齐 |
| hidden oracle 隔离无法硬沙箱 | denylist + isolation failure，不进入 E2 |
| prompt guard 误杀真实工程词 | context-sensitive 走 manual review，不自动 invalid |
| L1 TaskSpace 成本偏高 | 不修 prompt，记录 cost regression，作为机制问题 |

## 实施阶段

### Phase 0：脚手架与自测

产物：

- `scripts/taskspace-benchmark/` 基础目录。
- manifest parser。
- prompt guard。
- workspace 创建。
- harness self-test。

验收：

- `test-harness.ps1` 通过。
- 不调用 Whale。

### Phase 1：单场景 paired runner

产物：

- `run-taskspace-benchmark.ps1` 支持 `single-file-fast-fix`。
- left/right 运行 Whale。
- public validation + hidden oracle。
- pair report。

验收：

- `single-file-fast-fix` 生成 valid pair。
- hidden oracle isolation pass。

### Phase 2：迁移 order-pipeline

产物：

- `order-pipeline-growth` fixture。
- private oracle。
- taskspace observability export。
- graph health metrics。

验收：

- pair report 包含 standard/taskspace 对照。
- taskspace structural oracle 输出。
- 不要求 TaskSpace 一定优于 standard，只要求证据链完整。

### Phase 3：aggregate 与重复运行

产物：

- repeats 支持。
- aggregate report。
- rerun lineage。

验收：

- `-Repeats 3` 生成 3 个 pair。
- aggregate 展示 all/valid/excluded。

### Phase 4：回归集成

产物：

- `scripts/run-action-map-regression.ps1` 可选调用 benchmark smoke。
- CI/本地回归说明。

验收：

- 默认回归不跑长 E2E。
- 显式参数才跑 paired benchmark。

## 推荐提交切片

1. `Add TaskSpace benchmark harness scaffold`
2. `Add benchmark prompt guard and manifest validation`
3. `Add single-file paired benchmark scenario`
4. `Add order-pipeline paired benchmark scenario`
5. `Add benchmark aggregate reporting`

每个切片都应保持可运行，并提交对应自测或报告样本。

## 开始实施前检查清单

- 本地 `whale.exe` 可执行。
- `whale exec --help` 暴露 `--taskspace`。
- Python/pytest 可用。
- 现有 `export-action-map-observability.ps1` 可从 rollout/session 导出。
- git worktree clean。
- 不新建分支。
