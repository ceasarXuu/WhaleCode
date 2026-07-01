# R4 收益门禁和公开样本综合验收

> 本文补充 R4 的硬门禁：每个 phase 都必须证明具体工程收益；最后必须选取 10 个公开
> benchmark 样本做综合验收，逐样本分析 tools 调用质量和性能。没有证据的 phase 不能
> 标记完成。

## 4.1 补充门禁

```text
Gate R4-BENEFIT-PER-PHASE:
  every phase must provide concrete engineering benefit evidence.
  deliverable-only evidence is not enough.

Gate R4-PUBLIC-10-TOOL-STRESS:
  final acceptance must run 10 public benchmark samples selected for tool-call stress.
  samples must be traceable to public benchmark metadata, not invented local scenarios.

Gate R4-PUBLIC-10-PLAN-MANIFEST:
  the 10-sample selection and required report fields must be machine-readable.
  the plan gate can pass before the paired run, but R4-G cannot close until the paired run report also passes.

Gate R4-PUBLIC-10-LIVE-REGISTRY:
  the plan gate must fetch the public benchmark registry and prove that all 10 task ids belong to the pinned
  terminal-bench-core 0.1.1 task_id_subset.
  a local hard-coded allow-list is not sufficient evidence.
```

## 4.2 每个 phase 的收益证据格式

每个 phase 的 closeout 必须补一行 `PhaseBenefitEvidenceV1`：

```text
phase
claimed_engineering_benefit
baseline_artifact
after_artifact
measurement_method
metric_name
baseline_value
after_value
pass_threshold
pass_or_fail
residual_risk
evidence_paths
```

收益证据必须满足：

1. 有 baseline，不能只写“新增了能力”。
2. 有 after artifact，不能只写“代码已实现”。
3. 有 pass/fail 阈值，不能只写“有所改善”。
4. 对 correctness、performance、cache、observability 分开判断。
5. phase-local 可验证，不能依赖后续 phase 才证明当前 phase 完成。

## 4.3 Phase 收益门禁矩阵

| Phase | 必须证明的工程收益 | Baseline | After Evidence | Pass Threshold |
|---|---|---|---|---|
| R4-A | tool path 不再 unknown，新增 path 可被发现 | source audit 前 path matrix 为空或不完整 | `r4-tool-path-coverage.json` + `test-r4-tool-path-coverage.ps1` + `r4_tool_path_coverage` gate | unknown/unowned path = 0；missing source anchor = 0 |
| R4-B | 真实样本问题可分类、可追踪到证据 | R3 样本 scattered in target/CoE | `r4-sample-evidence-ledger.json` + `test-r4-sample-ledger.ps1` + `r4_sample_ledger` gate | known-bad 样本 100% 有 owner phase；primary evidence 全部存在 |
| R4-C | standard feedback、provider payload、TaskSpace map 共享同一语义源 | success/error/internal path 各自摘要 | envelope trace + payload proof + map proof | direct success/error/rejection fixture 通过 |
| R4-D | action-contract internal tool 失败可被 agent 下一轮看到并纠错 | `count-call-stack` wrong/no patch | rerun + exact payload proof | feedback-loss count = 0；样本 solved 或根因转移 |
| R4-E | projection/output-ref 不丢语义且不制造日志膨胀 | large-output timeout / 491MB rollout | large-output rerun + projection audit | no timeout；rollout size controlled；failure details retrievable |
| R4-F | non-direct tools 不再是盲区 | CodeMode/multi-agent/MCP attribution 未分类 | coverage fixture + exclusion proof | all non-direct paths classified；excluded paths have tests |
| R4-G | known-bad 和公开样本证明收益真实 | R3 sweep + selected public sample baseline | paired standard/taskspace run report | per-sample tool analysis complete；cache >= 0.95 |
| R4-H | 工程层收口可审计、可复现 | scattered phase evidence | closeout doc + committed artifacts | no phase marked completed without benefit evidence |

## 4.4 公开 benchmark 来源

当前 R4 最终验收优先使用 Terminal-Bench，因为本仓库已有 E3-equivalent adapter 和 source-guard
机制。公开来源：

```text
Benchmark: terminal-bench-core
Version: 0.1.1
Registry URL: https://raw.githubusercontent.com/laude-institute/terminal-bench/main/registry.json
Source URL: https://github.com/laude-institute/terminal-bench
Branch: dataset/terminal-bench-core/v0.1.x
Commit: 91e10457b5410f16c44364da1a34cb6de8c488a5
Dataset path: ./tasks
Registry subset count: 80
Registry task_id_subset sha256: c3a4e299ff002f3c2201de9dfdf0a7ed64a41cd1ea4253480d99502e086ce190
```

选择要求：

1. task id 必须存在于公开 registry 的 `terminal-bench-core` `0.1.1` `task_id_subset`。
2. 运行前必须写入 source metadata、registry checksum、task id 校验结果。
3. 不把 solution、hidden tests 或完整 benchmark dataset vendoring 到仓库。
4. 如果某个样本因本地环境不可运行而替换，替换样本也必须来自同一公开 registry，并记录替换原因。

机器可读计划和门禁：

```text
Plan manifest:
  docs/v0.0.5/build-R4/r4-public-10-tool-stress-plan.json
Plan gate:
  scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1
Default evidence:
  target/r4-public-10-tool-stress/r4-public-10-tool-stress-evidence.json
```

该门禁会检查样本数、task id 唯一性、公开来源元数据、选择理由和最终报告必填字段。默认运行时会在线读取
Terminal-Bench public registry，校验 `terminal-bench-core` `0.1.1` 的 `github_url`、`branch`、`commit_hash`、
`dataset_path`、80 个 `task_id_subset` 和 subset checksum；10 个候选样本必须全部属于该公开 subset。
带 `-ReportPath` 运行时，还会检查实际 10 样本结果表是否逐样本包含所有字段、样本是否属于计划集合，以及
`task_id_registry_verified=true`。

## 4.5 R4 公开 10 样本候选

这些样本全部来自 Terminal-Bench core `0.1.1` registry。选择标准是更容易触发 shell、
文件编辑、路径定位、测试执行、环境诊断、git 操作、输出解析和长流程工具反馈。

| # | Public Task ID | Tool-call Stress Focus | Why It Belongs In R4 |
|---|---|---|---|
| 1 | `vim-terminal-task` | script creation, text processing, command output validation | 考验文件创建、命令执行、stdout 对比和验证反馈；替换 `build-linux-kernel-qemu`，因为后者的官方测试运行时动态拉取 BusyBox 且没有 pinned hash，远程资产等价性无法证明，不能产生真实 paired tool-call 样本 |
| 2 | `heterogeneous-dates` | CSV inspection, numeric calculation, single-file answer artifact | 考验文件读取、数据解析、确定性计算、answer file 创建和验证反馈；替换 `qemu-alpine-ssh`，因为后者 Dockerfile 动态拉取 Alpine ISO 且没有 pinned hash |
| 3 | `sqlite-db-truncate` | SQLite recovery, binary/data inspection, JSON artifact validation | 考验数据库/文件工具、结构化输出生成、命令反馈和 validator 对比；替换 `qemu-startup`，因为后者和 `qemu-alpine-ssh` 共享未证明 Alpine ISO 远程资产 |
| 4 | `git-multibranch` | git state inspection, branch/file operations | 考验工具结果中的状态语义和路径归因 |
| 5 | `git-workflow-hack` | git history/workflow repair | 考验 command stderr/stdout 对后续决策的影响 |
| 6 | `organization-json-generator` | CSV inspection, schema reasoning, JSON generation and validation | 考验多文件读取、结构化转换、schema 约束输出和 validator feedback；替换 `sanitize-git-repo`，因为后者 `setup.sh` 需要未 pinned 的外部 git clone，无法形成可复现 paired run |
| 7 | `sqlite-with-gcov` | DB commands, compile/test coverage output | 考验结构化失败解析和验证工具反馈 |
| 8 | `processing-pipeline` | multi-step data/file pipeline | 考验中间产物、changed paths 和 test feedback |
| 9 | `csv-to-parquet` | data conversion, file validation | 考验文件检查、命令输出和 artifact validation |
| 10 | `tmux-advanced-workflow` | terminal/session workflow | 考验非平凡终端交互和工具调用时序 |

## 4.6 运行和分析门禁

综合验收至少运行一轮 paired standard/taskspace。每个样本必须输出一行报告：

```text
public_benchmark
benchmark_version
source_commit
task_id
task_id_registry_verified
standard_outcome
taskspace_outcome
standard_wall_time_ms
taskspace_wall_time_ms
taskspace_wall_time_ratio
standard_tool_calls
taskspace_tool_calls
taskspace_tool_call_ratio
standard_input_tokens
standard_output_tokens
taskspace_input_tokens
taskspace_output_tokens
taskspace_token_ratio
request_2_plus_cache_hit_rate
tool_feedback_loss_count
tool_feedback_semantic_loss_count
tool_result_projection_count_by_reason
taskspace_map_attribution_missing_count
large_output_ref_count
rollout_size_bytes
changed_paths_standard
changed_paths_taskspace
validation_result
failure_taxonomy
tool_call_analysis_summary
evidence_paths
```

工具调用分析必须逐样本回答：

1. tool 调用 intent 是否被正确执行或拒绝。
2. tool result 是否以 standard 等价语义进入下一轮 provider payload。
3. stderr/stdout/path/exit code 是否被保留、summary、ref 或丢失。
4. TaskSpace map 是否记录了同一语义，是否缺 attribution。
5. projection 是否 pair-safe，是否有 reason。
6. 重复失败是否形成语义进展，还是进入 loop。
7. TaskSpace 相比 standard 的 wall/token/tool-call 倍数是多少。

## 4.7 最终通过标准

R4 不能只要求 10 个样本全部 solved。通过标准分三层：

| Layer | Pass Standard |
|---|---|
| Tool chain correctness | `tool_feedback_loss_count = 0`，无 invalid tool-call history，provider payload proof 存在 |
| Engineering benefit | known-bad 样本不再因已知 feedback-loss/path 丢失而 wrong；公开样本中 tool failure 可被 agent 看到并响应 |
| Performance/cost | 无无界日志膨胀；cache hit `>= 0.95`；wall/token/tool-call 倍数逐样本报告，超过阈值必须有根因 |

如果公开样本失败，但证据显示失败来自模型解题错误或环境缺陷，而不是 TaskSpace tool 链路，
可以记录为 non-blocking correctness failure；但必须有 payload/map/tool feedback 证据支持，不能口头判断。
